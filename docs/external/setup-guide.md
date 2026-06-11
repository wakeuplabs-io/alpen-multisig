# Setup Guide

**Satisfies: PRD §1.4** — Installation via single command or double-click

## System Requirements

| Component | Minimum Requirement |
|-----------|---------------------|
| **Operating System** | Latest LTS release of Debian Linux, macOS, or Windows |
| **RAM** | 8 GB |
| **CPU** | 2 cores, 4 threads |
| **Storage** | 1 TB SSD |
| **Network** | 20 Mbps internet connection |

## Download the Application

Download the latest release from the [GitHub Releases page](https://github.com/wakeuplabs-io/alpen-multisig/releases).

Choose the appropriate installer for your platform:

| Platform | File | Installation Method |
|----------|------|---------------------|
| **Debian/Ubuntu** | `alpen-multisig_*.deb` | Double-click or `dpkg -i` |
| **Fedora/RHEL** | `alpen-multisig_*.rpm` | Double-click or `rpm -i` |
| **Linux (Universal)** | `alpen-multisig-*.AppImage` | `chmod +x` then run |
| **macOS** | `alpen-multisig-*.dmg` | Drag to Applications |
| **Windows** | `alpen-multisig-*.exe` | Double-click to launch |

## Installation

### Linux (Debian/Ubuntu)

**Graphical:**
1. Download the `.deb` file
2. Double-click the file to open it in your package manager
3. Click "Install"

**Command Line:**
```bash
sudo dpkg -i alpen-multisig_*.deb
```

### Linux (Fedora/RHEL)

**Graphical:**
1. Download the `.rpm` file
2. Double-click the file to open it in your package manager
3. Click "Install"

**Command Line:**
```bash
sudo rpm -i alpen-multisig-*.rpm
```

### Linux (AppImage)

The AppImage is a portable, universal Linux format that works on any distribution.

```bash
# Download the AppImage
wget https://github.com/wakeuplabs-io/alpen-multisig/releases/latest/download/alpen-multisig-*.AppImage

# Make it executable
chmod +x alpen-multisig-*.AppImage

# Run the application
./alpen-multisig-*.AppImage
```

### macOS

1. Download the `.dmg` file
2. Open the `.dmg` file
3. Drag the Alpen Multisig application to your Applications folder
4. Launch from Applications or Spotlight

**Note:** On first launch, you may need to right-click the application and select "Open" to bypass Gatekeeper security.

### Windows

1. Download the `.exe` file
2. Double-click the file to launch the application

**Prerequisites:**
- WebView2 runtime must be installed (included in Windows 10/11 by default)
- If WebView2 is not installed, download it from [Microsoft's website](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## First Run

### Step 1: Connect Your Hardware Wallet

1. Connect your supported hardware wallet (Trezor or Ledger) to your computer via USB
2. Unlock your hardware wallet
3. Launch the Alpen Multisig application
4. The application will detect your hardware wallet automatically

### Step 2: Select Your Address

1. The application will display a list of addresses derived from your hardware wallet
2. Select the address you want to use for signing (from the first 20 addresses on the `m/86'/0'/73'/0/n` derivation path)
3. Verify the address on your hardware wallet screen to ensure it matches

### Step 3: Select Your Multisig

1. Choose the multisig you want to interact with:
   - Alpen Administrator
   - Strata Administrator
   - Strata Sequencer Manager
   - Strata Security Council
   - Payout Administrator

### Step 4: Authenticate

1. Sign the authentication nonce with your hardware wallet
2. The application will verify your signature against the canonical signer list
3. You will be granted access to the multisig dashboard

## Configuration

### Bitcoin Node Connection

The application can connect to Bitcoin and Strata nodes in two ways:

**Option 1: Local Node (Default)**
- The application automatically detects a local Strata node running on your machine
- If no local node is detected, you will be prompted to start one or switch to a remote endpoint

**Option 2: Remote RPC Endpoint**
- Select "Use Remote RPC" in the connection settings
- Enter the RPC URL (e.g., `https://stratabtc.org` or your custom endpoint)
- The application will connect to the remote node

### Network Selection

The application supports both mainnet and testnet:
- **Mainnet:** For production governance operations
- **Testnet:** For testing and development

Select the network in the application settings before connecting your hardware wallet.

## Verifying Your Download

Before running the application, verify that your download is authentic and has not been tampered with.

### Step 1: Import Signing Keys

```bash
# Download the release signing keys
wget https://github.com/wakeuplabs-io/alpen-multisig/raw/main/release-keys/*.asc

# Import the keys
gpg --import *.asc
```

### Step 2: Verify the Signature

```bash
# Download the checksums and signature files
wget https://github.com/wakeuplabs-io/alpen-multisig/releases/latest/download/SHA256SUMS
wget https://github.com/wakeuplabs-io/alpen-multisig/releases/latest/download/SHA256SUMS.*.asc

# Verify the signature
gpg --verify SHA256SUMS.*.asc SHA256SUMS
```

You should see `Good signature from "<signer name>"` in the output.

### Step 3: Verify the Checksum

```bash
# Verify your download against the checksums
sha256sum --ignore-missing -c SHA256SUMS
```

You should see `OK` next to the file you downloaded.

See [Verifying Releases](./verifying-releases.md) for detailed verification instructions.

## Updating the Application

The application does not currently support automatic updates. To update:

1. Download the latest release from GitHub
2. Verify the download (see above)
3. Install the new version, which will replace the existing installation
4. Your configuration and data will be preserved

## Troubleshooting

### Hardware Wallet Not Detected

- Ensure your hardware wallet is connected via USB and unlocked
- Check that your user account has permission to access USB devices (Linux: add your user to the `plugdev` group)
- Try a different USB port or cable
- Restart the application after connecting the hardware wallet

### Cannot Connect to Bitcoin Node

- Verify that your Bitcoin/Strata node is running and accessible
- Check that the RPC URL is correct and includes the proper protocol (`http://` or `https://`)
- Ensure your firewall allows connections to the RPC port
- If using a remote endpoint, verify that the endpoint is operational

### Application Crashes on Launch

- Ensure your system meets the minimum requirements
- On Linux, check that you have the required dependencies installed
- On macOS, ensure you have granted the necessary permissions (Accessibility, Full Disk Access if needed)
- On Windows, ensure WebView2 runtime is installed
- Check the application logs for detailed error messages

### Signature Verification Fails

- Ensure you have imported the correct signing keys
- Verify that you downloaded all files from the same release
- Check that the signature file matches the checksums file
- Re-download the files if verification continues to fail

## Support

For issues, questions, or feedback:

- **GitHub Issues:** [https://github.com/wakeuplabs-io/alpen-multisig/issues](https://github.com/wakeuplabs-io/alpen-multisig/issues)
- **Documentation:** See the [README](./README.md) for links to all documentation

## Related Documents

- [Verifying Releases](./verifying-releases.md) — Detailed verification instructions
- [Reproducible Builds](./reproducible-builds.md) — Independent build verification
- [Build and Release Process](./build-and-release-process.md) — Overview of the release pipeline
