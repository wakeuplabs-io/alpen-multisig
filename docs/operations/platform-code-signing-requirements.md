# Platform Code Signing Requirements

> **External document — For Alpen Labs**

## Overview

Platform code signing is a security mechanism used by operating systems to verify the identity of the software publisher and ensure that applications have not been tampered with since they were signed. When properly signed, users see the trusted publisher's name and can install applications without security warnings.

This document outlines the platform code signing requirements for distributing the Strata Multisig application on macOS and Windows.

## macOS — Apple Developer ID

### What It Enables

When the application is signed with an Apple Developer ID and notarized by Apple, users can install and run the application without encountering Gatekeeper warnings. The application appears to come from an identified developer, providing a seamless installation experience.

Without this signing, users encounter the following message when attempting to open the downloaded application:

> "App cannot be opened because it was downloaded from an unidentified developer."

The user must then manually navigate to System Settings > Privacy & Security and explicitly allow the application to run, which creates a poor user experience and may cause users to perceive the application as untrustworthy.

### Requirements

To obtain an Apple Developer ID for code signing, the following is required:

- An active **Apple Developer Program** membership ($99 USD per year)
- An organization identity (if signing on behalf of a company)
- A D-U-N-S Number for the organization, obtainable from Dun & Bradstreet

### Process Summary

1. Enroll in the Apple Developer Program
2. Complete organization verification with Apple
3. Generate a Developer ID Application certificate
4. Configure code signing in the build pipeline
5. Submit the application for Apple's notarization service

### Notes

- The Developer Program enrollment and certificate generation must be performed by the organization (Alpen Labs) directly through Apple's developer portal.
- Notarization is required for distribution outside the Mac App Store.
- The code signing identity must be renewed annually.

## Windows — Authenticode

### What It Enables

When the application is signed with an Authenticode certificate from a trusted Certificate Authority (CA), Windows SmartScreen recognizes the publisher as trusted. Users can download and run the application without encountering security warnings.

Without this signing, users encounter the following when attempting to run the downloaded installer:

> "Windows Defender SmartScreen prevented an unrecognized app from starting."

The user must click "More info" and then "Run anyway," which creates a poor user experience and may cause users to perceive the application as dangerous.

### Requirements

To obtain an Authenticode code signing certificate, the following is required:

- A code signing certificate issued by a trusted Certificate Authority (CA)
- Organization identity validation by the CA
- The certificate private key secured appropriately

### Certificate Authorities

Code signing certificates can be obtained from any trusted CA, including:

- DigiCert
- GlobalSign
- Sectigo (formerly Comodo)

### Process Summary

1. Generate a Certificate Signing Request (CSR) from a secure system
2. Submit the CSR to a trusted CA along with organization verification documents
3. Complete the CA's identity verification process
4. Receive the signed certificate
5. Configure the certificate in the build pipeline for signing binaries

### Notes

- Extended Validation (EV) certificates provide immediate trust reputation in Windows SmartScreen, whereas standard certificates may require a reputation-building period.
- The code signing certificate must be renewed before expiration.
- The private key must be stored securely and never committed to version control.

## Current Implementation Status

| Platform | PGP Cryptographic Signing | Platform-Level Code Signing |
|----------|---------------------------|----------------------------|
| Linux | Implemented | Not applicable |
| macOS | Implemented | Pending — requires Apple Developer Program enrollment |
| Windows | Implemented | Pending — requires Authenticode certificate |

Cryptographic signing (PGP) is currently implemented for all platforms and provides integrity verification. Platform-level code signing is a complementary layer that provides operating system trust and a better user experience.

## Action Required

Platform code signing for macOS and Windows requires the following from Alpen Labs:

- **macOS:** Enroll in the Apple Developer Program and obtain a Developer ID certificate
- **Windows:** Obtain an Authenticode code signing certificate from a trusted CA

These actions are outside the scope of the current development phase and are documented here for planning purposes. When Alpen Labs is ready to proceed, the development team can integrate the required certificates into the release pipeline.

## References

- Apple Developer Program: https://developer.apple.com/programs/
- Microsoft Code Signing: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/authenticode
- Trusted CA List (Microsoft): https://learn.microsoft.com/en-us/windows-hardware/drivers/installRoot-the-list-of-trusted-root-certification-authorities-in-windows