#!/usr/bin/env bash
#
# Server Setup Script - Phased Deployment
# Usage: ./startup.sh --phase 1|2|3|4|5|6
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info()    { echo -e "${BLUE}[+]${NC} $*"; }
log_success() { echo -e "${GREEN}[✓]${NC} $*"; }
log_warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
log_error()   { echo -e "${RED}[✗]${NC} $*" >&2; }

# Usage function
usage() {
    cat << EOF
Usage: $(basename "$0") --phase <1|2|3|4|5|6>

Server Setup Script - Phased Deployment

OPTIONS:
    --phase <1|2|3|4|5|6>  Run specific setup phase

PHASES:
    1   Create user account (akinus)
    2   Harden SSH configuration
    3   Install WireGuard VPN
    4   Configure UFW firewall
    5   Mount storage box & configure swap
    6   Install Docker

EXAMPLES:
    $(basename "$0") --phase 1
    $(basename "$0") --phase 2
    $(basename "$0") --phase all

EOF
    exit 1
}

# Verify running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

# ============================================================================
# PHASE 1: Create User Account
# ============================================================================
phase1() {
    log_info "=========================================="
    log_info "  PHASE 1: Creating User Account"
    log_info "=========================================="
    echo

    local username="akinus"

    # Check if user already exists
    if id "$username" &>/dev/null; then
        log_warn "User '$username' already exists. Skipping user creation."
    else
        log_info "Creating user: $username"
        useradd -m -s /bin/bash "$username"
        log_success "User '$username' created"
    fi

    # Set password
    log_info "Setting password for '$username'"
    passwd "$username"

    # Add to sudo group
    log_info "Adding '$username' to sudo group"
    usermod -aG sudo "$username"

    log_success "User account setup complete"
    echo
    log_warn "=========================================="
    log_warn "  IMPORTANT: SSH KEY SETUP REQUIRED"
    log_warn "=========================================="
    log_warn "Before continuing to Phase 2, you must:"
    echo
    echo "  1. From your HOME machine, copy your SSH public key:"
    echo "     ssh-copy-id $username@<server-ip>"
    echo
    echo "  2. Test SSH access from your home box:"
    echo "     ssh $username@<server-ip>"
    echo
    read -rp "Press ENTER after you have tested SSH access to continue: "
    echo
}

# ============================================================================
# PHASE 2: Harden SSH Configuration
# ============================================================================
phase2() {
    log_info "=========================================="
    log_info "  PHASE 2: Hardening SSH Configuration"
    log_info "=========================================="
    echo

    # Backup SSH config
    log_info "Backing up SSH config..."
    cp /etc/ssh/sshd_config /etc/ssh/sshd_config.bak
    log_success "Backup saved to /etc/ssh/sshd_config.bak"

    # Disable root login
    log_info "Disabling root login..."
    grep -q '^PermitRootLogin' /etc/ssh/sshd_config \
        && sed -i 's/^PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config \
        || echo 'PermitRootLogin no' >> /etc/ssh/sshd_config

    # Disable password authentication
    log_info "Disabling password authentication..."
    grep -q '^PasswordAuthentication' /etc/ssh/sshd_config \
        && sed -i 's/^PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config \
        || echo 'PasswordAuthentication no' >> /etc/ssh/sshd_config

    # Disable keyboard-interactive auth
    log_info "Disabling keyboard-interactive authentication..."
    grep -q '^KbdInteractiveAuthentication' /etc/ssh/sshd_config \
        && sed -i 's/^KbdInteractiveAuthentication.*/KbdInteractiveAuthentication no/' /etc/ssh/sshd_config \
        || echo 'KbdInteractiveAuthentication no' >> /etc/ssh/sshd_config

    # Ensure public key auth is enabled
    log_info "Ensuring public key authentication is enabled..."
    grep -q '^PubkeyAuthentication' /etc/ssh/sshd_config \
        && sed -i 's/^PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config \
        || echo 'PubkeyAuthentication yes' >> /etc/ssh/sshd_config

    # Restrict SSH access to the akinus account
    log_info "Restricting SSH access to akinus account..."
    grep -q '^AllowUsers' /etc/ssh/sshd_config \
        && sed -i 's/^AllowUsers.*/AllowUsers akinus/' /etc/ssh/sshd_config \
        || echo 'AllowUsers akinus' >> /etc/ssh/sshd_config

    # Validate config
    log_info "Validating SSH configuration..."
    if sshd -t 2>&1; then
        log_success "SSH configuration is valid"
    else
        log_error "SSH configuration validation failed!"
        exit 1
    fi

    # Reload SSH
    log_info "Reloading SSH service..."
    systemctl reload ssh 2>/dev/null || systemctl reload sshd 2>/dev/null
    log_success "SSH service reloaded"

    echo
    log_warn "=========================================="
    log_warn "  IMPORTANT: TEST SSH BEFORE CONTINUING"
    log_warn "=========================================="
    log_warn "Open another terminal and test SSH access:"
    echo
    echo "  ssh akinus@<server-ip>"
    echo
    echo "If you cannot connect, DO NOT proceed. Restore with:"
    echo "  cp /etc/ssh/sshd_config.bak /etc/ssh/sshd_config"
    echo "  systemctl reload ssh"
    echo
    read -rp "Press ENTER after you have tested SSH access: "
    echo
    log_success "Phase 2 complete"
    echo
}

# ============================================================================
# PHASE 3: Install WireGuard VPN
# ============================================================================
phase3() {
    log_info "=========================================="
    log_info "  PHASE 3: Installing WireGuard VPN"
    log_info "=========================================="
    echo

    # Install WireGuard
    log_info "Installing WireGuard..."
    apt update
    apt install -y wireguard wireguard-tools
    log_success "WireGuard installed"

    # Verify installation
    log_info "Verifying installation..."
    wg --version
    echo

    # Generate keys
    log_info "Generating WireGuard keypair..."
    cd ~
    umask 077
    wg genkey | tee private.key | wg pubkey > public.key
    PRIVATE_KEY=$(cat private.key)
    log_success "Keys generated"

    echo
    echo "=========================================="
    echo "  YOUR PUBLIC KEY (for hub server)"
    echo "=========================================="
    cat public.key
    echo "=========================================="
    echo

    # Get host number
    read -rp "Enter the new WireGuard host number (e.g. 10 for 10.200.200.10): " WG_HOST
    echo

    # Create WireGuard config
    log_info "Creating /etc/wireguard/wg0.conf..."
    cat >/etc/wireguard/wg0.conf <<EOF
[Interface]
PrivateKey = ${PRIVATE_KEY}
Address = 10.200.200.${WG_HOST}/24
ListenPort = 51820

[Peer]
PublicKey = IOSw0+CgnAQBjh4vTQ6oFLJarkJuBxR326SDLTf8IBE=
Endpoint = 178.105.18.55:51820
AllowedIPs = 10.200.200.0/24
PersistentKeepalive = 25
EOF

    chmod 600 /etc/wireguard/wg0.conf
    log_success "Configuration created"

    # Enable and start WireGuard
    log_info "Enabling WireGuard service..."
    systemctl enable wg-quick@wg0

    log_info "Starting WireGuard..."
    systemctl restart wg-quick@wg0

    echo
    log_info "Current WireGuard Status:"
    wg show
    echo

    echo "=========================================="
    echo "  ADD THIS PEER TO THE HUB SERVER"
    echo "=========================================="
    echo
    echo "Peer configuration for hub:"
    echo
    echo "[Peer]"
    echo "PublicKey = $(cat public.key)"
    echo "AllowedIPs = 10.200.200.${WG_HOST}/32"
    echo "Endpoint = <SERVER_IP>:51820"
    echo "PersistentKeepalive = 25"
    echo
    echo "=========================================="
    echo
    echo "Run the following command to restart wireguard..."
    echo "sudo systemctl restart wg-quick@wg0"

    read -rp "Press ENTER after the peer has been added to the hub..."

    echo
    log_info "Testing connectivity..."
    ping -c 4 10.200.200.1 || true
    echo

    log_info "WireGuard handshake information:"
    wg show
    echo

    # Clean up raw key files now that the private key is embedded (with chmod 600) in wg0.conf
    rm -f ~/private.key ~/public.key

    log_success "Phase 3 complete"
    echo
}

# ============================================================================
# PHASE 4: Configure UFW Firewall
# ============================================================================
phase4() {
    log_info "=========================================="
    log_info "  PHASE 4: Configuring UFW Firewall"
    log_info "=========================================="
    echo

    # Install ufw
    log_info "Installing ufw..."
    apt update
    apt install -y ufw
    log_success "ufw installed"

    # Set default policies
    log_info "Setting default policies (deny incoming, allow outgoing)..."
    ufw default deny incoming
    ufw default allow outgoing

    # Allow SSH
    log_info "Allowing SSH (22/tcp)..."
    ufw allow 22/tcp comment 'SSH'

    # Allow WireGuard
    log_info "Allowing WireGuard (51820/udp)..."
    ufw allow 51820/udp comment 'WireGuard'

    # Enable ufw (--force skips the interactive y/n prompt)
    log_info "Enabling ufw..."
    ufw --force enable

    echo
    log_info "Active rules:"
    ufw status verbose
    echo

    log_success "Firewall configuration complete"
    echo
    log_warn "=========================================="
    log_warn "  ALLOWED SERVICES:"
    log_warn "=========================================="
    echo "  - SSH (22/tcp)"
    echo "  - WireGuard (51820/udp)"
    echo "  - Established/related connections (stateful by default)"
    echo "  - ICMP/ping (allowed by ufw by default)"
    echo
    log_warn "ALL OTHER INGRESS IS BLOCKED"
    echo
    log_warn "IMPORTANT: Make sure SSH is working before proceeding!"
    echo
}

# ============================================================================
# PHASE 5: Mount Storage Box & Configure Swap
# ============================================================================
phase5() {
    log_info "=========================================="
    log_info "  PHASE 5: Storage Box Mount & Swap"
    log_info "=========================================="
    echo

    # --- Storage Box SSH key ---
    log_warn "=========================================="
    log_warn "  STORAGE BOX SSH KEY SETUP"
    log_warn "=========================================="
    echo "This server needs the storage_rsa keypair to mount the Hetzner Storage Box."
    echo "If this keypair already exists elsewhere, copy its contents into the files below."
    echo
    echo "  1. In another terminal, create the private key:"
    echo "     nano ~/.ssh/storage_rsa"
    echo "     (paste the private key content, save and exit)"
    echo
    echo "  2. Create the public key:"
    echo "     nano ~/.ssh/storage_rsa.pub"
    echo "     (paste the public key content, save and exit)"
    echo
    read -rp "Press ENTER once both ~/.ssh/storage_rsa and ~/.ssh/storage_rsa.pub exist: "
    echo

    if [[ ! -f ~/.ssh/storage_rsa ]] || [[ ! -f ~/.ssh/storage_rsa.pub ]]; then
        log_error "storage_rsa key files not found in ~/.ssh/. Aborting."
        exit 1
    fi

    chmod 600 ~/.ssh/storage_rsa
    chmod 644 ~/.ssh/storage_rsa.pub
    log_success "Storage key files found, permissions set"
    echo

    # --- Storage Box mount ---
    read -rp "Enter the Storage Box sub-account username (e.g. u583127-sub1): " SB_USER
    read -rp "Enter the Storage Box hostname (e.g. u583127.your-storagebox.de): " SB_HOST

    log_info "Authorizing key on the Storage Box..."
    ssh -p23 -i ~/.ssh/storage_rsa -o StrictHostKeyChecking=accept-new "${SB_USER}@${SB_HOST}" install-ssh-key < ~/.ssh/storage_rsa.pub

    log_info "Installing sshfs..."
    apt update
    apt install -y sshfs

    log_info "Mounting Storage Box..."
    mkdir -p /mnt/storagebox-services
    sshfs -o allow_other,default_permissions,IdentityFile=/root/.ssh/storage_rsa,IdentitiesOnly=yes,StrictHostKeyChecking=accept-new \
        -p23 "${SB_USER}@${SB_HOST}:/" /mnt/storagebox-services

    log_info "Creating services folder structure..."
    mkdir -p /mnt/storagebox-services/services/linkding
    mkdir -p /mnt/storagebox-services/services/vaultwarden
    mkdir -p /mnt/storagebox-services/services/akocloud
    log_success "Folders created:"
    ls -la /mnt/storagebox-services/services/

    log_info "Adding persistent mount to /etc/fstab..."
    if ! grep -q "storagebox-services" /etc/fstab; then
        echo "${SB_USER}@${SB_HOST}:/ /mnt/storagebox-services fuse.sshfs noauto,x-systemd.automount,_netdev,users,idmap=user,IdentityFile=/root/.ssh/storage_rsa,IdentitiesOnly=yes,allow_other,reconnect 0 0" >> /etc/fstab
        systemctl daemon-reload
        log_success "fstab entry added"
    else
        log_warn "fstab already contains a storagebox-services entry, skipping"
    fi

    echo
    log_info "Mount status:"
    df -h | grep storagebox || log_warn "Mount not showing in df -h yet — check manually"
    echo

    # --- Swap ---
    log_info "Configuring swap..."
    if swapon --show=NAME 2>/dev/null | grep -q '/swapfile'; then
        log_warn "/swapfile is already active, skipping swap setup"
    else
        fallocate -l 4G /swapfile
        chmod 600 /swapfile
        mkswap /swapfile
        swapon /swapfile
        if ! grep -q '/swapfile' /etc/fstab; then
            echo '/swapfile none swap sw 0 0' >> /etc/fstab
        fi
        log_success "4G swapfile created and enabled"
    fi

    echo
    log_info "Swap status:"
    swapon --show
    free -h
    echo

    log_success "Phase 5 complete"
    echo
}

# ============================================================================
# PHASE 6: Install Docker
# ============================================================================
phase6() {
    log_info "=========================================="
    log_info "  PHASE 6: Installing Docker"
    log_info "=========================================="
    echo

    # Remove old Docker versions
    log_info "Removing old Docker versions (if any)..."
    apt remove -y docker docker-engine docker.io containerd runc 2>/dev/null || true

    # Install dependencies
    log_info "Installing dependencies..."
    apt update
    apt install -y ca-certificates curl gnupg lsb-release

    # Add Docker GPG key
    log_info "Adding Docker GPG key..."
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    chmod a+r /etc/apt/keyrings/docker.gpg

    # Add Docker repository
    log_info "Adding Docker repository..."
    . /etc/os-release

    echo \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
    https://download.docker.com/linux/ubuntu \
    ${VERSION_CODENAME} stable" \
    > /etc/apt/sources.list.d/docker.list

    # Install Docker Engine
    log_info "Installing Docker Engine..."
    apt update
    apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

    # Enable and start Docker
    log_info "Enabling Docker service..."
    systemctl enable docker
    systemctl start docker

    # Add user to docker group
    log_info "Adding user 'akinus' to docker group..."
    usermod -aG docker akinus

    # Verify installation
    echo
    log_info "Verifying Docker installation..."
    docker version
    docker compose version
    echo

    log_info "Testing hello-world container..."
    docker run --rm hello-world

    echo
    log_success "=========================================="
    log_success "  Docker installation complete!"
    log_success "=========================================="
    echo
    log_warn "IMPORTANT: Log out and back in for docker group permissions to apply"
    echo
}

# ============================================================================
# Main Script
# ============================================================================
main() {
    # Parse arguments
    local phase=""

    if [[ $# -eq 0 ]]; then
        usage
    fi

    while [[ $# -gt 0 ]]; do
        case $1 in
            --phase)
                phase="$2"
                shift 2
                ;;
            --help|-h)
                usage
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                ;;
        esac
    done

    # Validate phase
    case "$phase" in
        1) check_root; phase1 ;;
        2) check_root; phase2 ;;
        3) check_root; phase3 ;;
        4) check_root; phase4 ;;
        5) check_root; phase5 ;;
        6) check_root; phase6 ;;
        all)
            check_root
            phase1
            phase2
            phase3
            phase4
            phase5
            phase6
            ;;
        *)
            log_error "Invalid phase: $phase"
            echo "Valid options: 1, 2, 3, 4, 5, 6, or 'all'"
            exit 1
            ;;
    esac
}

main "$@"
