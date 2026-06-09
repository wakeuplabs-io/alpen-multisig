```
// https://alpenlabs.notion.site/PRD-Strata-multisig-app-2c4901ba000f805eb6c4d0d62bb4e74f
```

# PRD: Strata multisig app

**Background**

The Strata protocol specifies several multisigs for administrative functions, namely the Strata Administrator, the Strata Sequencer Manager, the Strata Security Council, and the Payout Administrator. Additionally, the Alpen protocol specifies an administrative multisig, the Alpen Administrator. This document specifies requirements for a minimal, cross-platform desktop application ("the application") that enables easy and secure management of these multisigs by their respective signers.

**Additional references and resources**

- [Strata Multisig Backend - Design Guidelines & Architectural Notes](https://alpenlabs.notion.site/External-copy-Strata-Multisig-Backend-Design-Guidelines-Architectural-Notes-317901ba000f8064bfc4cd7433e1261f)
- [SPS-50: L1 transaction header and interpretation](https://www.notion.so/317901ba000f800c8f5ee39d810b159a?pvs=25)
- [SPS-51: Generic simple envelope format](https://alpenlabs.notion.site/External-copy-SPS-51-Generic-simple-envelope-format-317901ba000f809caeaadab8951ddf17)
- [SPS-65: Strata administration subprotocol (transaction processing subsection)](https://alpenlabs.notion.site/SPS-65-Strata-administration-subprotocol-Transaction-processing-subsection-317901ba000f80bf8d96eb5ef0667772)
- [Relevant block payouts transactions](https://alpenlabs.notion.site/External-copy-Block-payouts-367901ba000f80aa9217c51c8092b1ca?source=copy_link)

**Roles**

Bridge Operators: a user whose public key was included in the canonical list of bridge operators at the time of a Strata bridge deposit.
Alpen Administrator Signer: a user whose public key is listed in the Alpen consensus protocol as one of the signer keys on the Alpen Administrator multisig.
Strata Administrator Signer: a user whose public key is listed in the Strata consensus protocol as one of the signer keys on the Strata Administrator multisig.
Strata Sequencer Manager Signer: a user whose public key is listed in the Strata consensus protocol as one of the signer keys on the Strata Sequencer Manager multisig.
Strata Security Council Signer: a user whose public key is listed in the Strata consensus protocol as one of the signer keys on the Strata Security Council multisig.
Payout Administrator Signer: a user who public key is included as a spender of the block_payouts transaction in the Strata bridge bitcoin script.
Note: Where the term "user" is used without further specification, the requirement applies to all roles described above.

**Requirements**

1.  1. The user MUST be able to run the application locally on their desktop on an up-to-date version of the latest Long Term Support (LTS) release of the Debian Linux, Mac, or Windows operating system, using computer hardware with a minimum of 8 GB RAM, 2c4t CPU, 1 TB SSD, and 20 Mbps internet.
    2. Builds of the application MUST be [reproducible](https://reproducible-builds.org/docs/definition/).
    3. The user SHOULD be able to cryptographically verify that the application binary they are running was published and approved by multiple employees of Alpen Labs.
    4. The user MUST be able to install or run the application with either a single terminal command or double-click on an application icon.
       1. The installation of dependencies MUST take no more than one additional command or click, if any additional steps are required at all for installing dependencies.
       1. Any clicks required to approve administrative privileges for installing the application can be disregarded for this requirement.
2.  1. The application MUST support accessing read/write functionality for bitcoin and Strata using either a trusted RPC endpoint or by connecting to a Strata node running locally on the same desktop.
       1. The user MUST be able to select a trusted RPC endpoint run on the [https://stratabtc.org](https://stratabtc.org/) domain for accessing read/write functionality for bitcoin and Strata, or enter their own custom RPC URL.
       1. The default connection method MUST be connecting to a node running locally on the same desktop.
          1. If no local node is detected, then the user MUST be prompted to either turn on their local node or switch the connection method to use a trusted RPC endpoint.
       1. If the user is running a local Strata node, the application SHOULD be able to access read/write functionality for bitcoin and Strata using that node with no additional effort from the user.
3.  1. The user MUST be able to select the multisig they want to interact with. The supported multisigs MUST be:
       1. Alpen Administrator multisig. MUST be usable exclusively by all Alpen Administrator Signers.
       1. Strata Administrator multisig. MUST be usable exclusively by all Strata Administrator Signers
       1. Strata Sequencer Manager multisig. MUST be usable exclusively by all Strata Sequencer Manager Signers.
       1. Strata Security Council multisig. MUST be usable exclusively by all Strata Security Council Signers.
       1. Payout Administrator multisig. MUST be usable exclusively by all Payout Administrator signers.
    2. The user MUST be able to connect a supported hardware wallet and see a list of addresses to choose from.
       1. Supported hardware wallets MUST include all hardware wallets currently supported by [HWI](https://github.com/bitcoin-core/HWI/blob/master/docs/devices/index.rst) using the following features:
          - Taproot inputs,
          - Message signing,
          - Display on device screen, and
          - Are otherwise compatible with SPS-65 updates.
       1. Admin ID: 
          1. If the user selected the Payout Administrator in the previous step, then the application MUST use the user’s connected hardware wallet to generate an “Admin ID” represented by a P2TR bitcoin address generated using the [BIP-86-compliant](https://bips.dev/86/) m/86'/0'/73'/0/0 derivation path, where 73' is the hardened account used to generate the Admin ID, and /0/0 are the change and address indexes, respectively. The Admin ID MUST be used for authenticating with the multisig app backend and signing all Payout Administrator transactions, as described in Requirement 6.
          1. If the user selected any multisig other than the Payout Administrator in the previous step, then the application MUST use the user’s connected hardware wallet to generate an “Admin ID” represented by a P2WPKH bitcoin address generated using the [BIP-84-compliant](https://bips.dev/84/) m/84'/0'/73'/0/0 derivation path of the connected hardware wallet, where 73' is the hardened account used to generate the Admin ID, and 0 is the address index. The Admin ID MUST be used for authenticating with the multisig app backend and signing all admin subprotocol update-related messages. For the avoidance of doubt, the Admin ID MUST NOT be used to sign any bitcoin transactions.
       1. Admin Wallet: The application MUST use the user’s connected hardware wallet to generate an “Admin Wallet” using the [BIP-86-compliant](https://bips.dev/86/) m/86'/0'/73'/n/n derivation path, where 73' is the hardened account used to generate the Admin Wallet, and /n/n are the change and address indexes, respectively.I
       1. The user MUST be able to clearly read and understand each message they are signing on their hardware wallet screen, to be able to visually verify that the message they are signing matches what they are expecting based on what they are seeing in the application UI.
    3. After connecting their Admin ID and Admin Wallet, the user MUST sign a nonce with the private key of their Admin ID to gain access to the UI for the selected multisig.
       1. The user MUST only be given access to the UI for the selected multisig if the address that they have signed the nonce for is on the canonical list of multisig signers.
       1. If the user produces an invalid signature, then the user SHOULD be shown an error message saying so.
       1. If the user produces a valid signature, but the connected Admin ID is not on the canonical list of signers on the selected multisig, then the user SHOULD be shown an error message saying so.
    4. After gaining access to view the selected multisig, the user MUST be able to close the selected multisig UI and go back to the multisig selection screen.
    5. After gaining access to view the selected multisig, the user MUST be able to disconnect the selected address and go back to the wallet connection screen.
4.  1. After logging in to the application, the user MUST be able to see their Admin ID and copy it to the clipboard.
    2. The user MUST be able to use the application to view their Admin ID on their hardware wallet screen to verify that the Admin ID they see in the UI was actually derived from the seed phrase loaded in their connected hardware wallet.
    3. The user MUST be able to manage their Admin Wallet using the application, including:
       1. Balance: The user MUST be able to see the total BTC balance of the wallet (net of unconfirmed send and receive transactions) and the net balance of all unconfirmed send and receive transactions.
       1. Addresses: The user MUST be able to see each address in the wallet that holds a balance along with the current balance of each address (net of unconfirmed transactions).
       1. Transactions: The user MUST be able to see each unconfirmed transaction sent from the Admin Wallet and have the ability to bump the fee 
       1. Receive: see the first unused address in the wallet address index, in both text and QR code formats.
          1. Clicking on the address text or QR code MUST copy the address to the user’s clipboard.
          1. The user MUST be able to click a button to view their receive address on their hardware wallet screen to verify that the address they see in the UI was actually derived from the seed phrase loaded in their connected hardware wallet.
          1. Each address SHOULD be “one-time use” i.e. after the user has received BTC in a given address, the app MUST automatically rotate the address on the “receive” screen to show a different, unused address.
       1. Send BTC:
          1. Send to: The user MUST be able to enter any standard bitcoin address to send to i.e. P2PK, P2PKH, P2SH, P2WPKH, P2WSH, P2TR. If the user enters an output type that is not standard or consensus valid, then the user MUST be shown a critical error message: "Destination must be a bitcoin address." If the user enters a destination address that is not a bitcoin address on the correct network (e.g. the user enters a testnet bitcoin address when the wallet is currently connected to bitcoin mainnet) then the user MUST be shown a critical error message: "Destination must be a [correct network] bitcoin address." where [correct network] says mainnet or testnet, depending on which network the wallet is connected to.
          1. Amount: The user MUST be able to specify any valid amount of BTC to send. A “valid amount” is any amount where amount ≤ wallet balance - (fee rate s/vB * transaction size vB) , or click a “Max” button to automatically enter the maximum allowed amount. If the user enters a send amount that results in send amount + mining fee > wallet balance , then the user MUST be shown a critical error message: “Insufficient funds”.
          1. The user MUST be able to manually specify the fee rate of their send transaction in s/VB, in increments of 0.1 s/VB up to 10,000 s/vB. The default fee rate shown MUST be the “next block” fee rate provided by the Bitcoin Core node that the application is connected to.
          1. Change from the send transaction (if any) MUST be sent to the first unused address in the wallet’s change index.
          1. There MUST be a "Confirm" button that is disabled by default, and is only enabled when all send form text entry fields are correctly filled.
             1. Clicking the "Confirm" button when it is enabled MUST send a bitcoin transaction to the user’s connected hardware wallet; if the user confirms the transaction on their hardware wallet, then the transaction MUST be sent to the bitcoin network for confirmation and the user MUST be shown the transaction ID; if the user rejects the transaction in their hardware wallet then nothing should happen in the UI and the user can either click “Confirm” to try again or else back out of the “send” screen.
5.  1. The requirements in this section MUST only apply to the following multisigs, unless explicitly stated otherwise:
       - Alpen Administrator multisig
       - Strata Administrator multisig
       - Strata Sequencer Manager multisig
       - Strata Security Council multisig
    2. The user MUST be able to see all "Approved" updates and how many cancellation signatures each "Approved" update has received (if any). An "Approved" update is an update that has reached the required quorum of approval signatures and has been confirmed onchain, but has not yet been enacted.
       1. The user MUST be able to cancel any "Approved" update.
          1. The user MUST be able to copy all available cancellation signatures for a given update to their clipboard.
          1. The user MUST be able to create a cancellation transaction for a given "Approved" update, paste in the quorum of signatures required to cancel the update, and broadcast the cancellation transaction to bitcoin for confirmation either using the application's bitcoin RPC or by copying the raw transaction to the clipboard and broadcasting the transaction through any other bitcoin RPC.
          1. Canceled updates MUST be kept offchain and accessible/visible only to multisig signers.
       1. For the avoidance of doubt, this subsection does not apply to the following multisigs, because they do not produce update types that have an "Approved" or "Canceled" state:
          - Strata Sequencer Manager multisig
          - Strata Security Council multisig
    3. The user MUST be able to see all "Pending" updates, including how much time is left before the "Pending" update expires and how many approval signatures the "Pending" update has received (if any) out of the total number of required signatures. A "Pending" update is an update that has been proposed but has not yet reached the required quorum of signatures for approval and been confirmed on bitcoin.
       1. All "Pending" updates MUST be kept offchain and accessible/visible only to multisig signers.
       1. The user MUST be able produce an approval signature for any "Pending" update.
          1. The user MUST be able to copy all available approval signatures for a given "Pending" update to their clipboard.
          1. The user MUST be able to create an approval transaction for a given "Pending" update, paste in the quorum of signatures required to approve the update, and broadcast the approval transaction to bitcoin for confirmation either using the application's bitcoin RPC or by copying the raw transaction to the clipboard and broadcasting the transaction through any other bitcoin RPC. This flow SHOULD have a UI/UX similar to the “send” screen in the “wallet” section.
          1. The user whose signature causes the update transaction to reach its quorum SHOULD be given the option of creating, signing, and broadcasting the bitcoin transaction necessary for the update to be confirmed on bitcoin, or declining to do so.
             1. "Pending" updates that have reached quorum but have not been confirmed yet MUST have a "Send" button that, when clicked, enable the user to create, sign, and broadcast the bitcoin transactions necessary for the update to be confirmed on bitcoin, with a UI/UX similar to the “send” screen in the “wallet” section.
       1. A "Pending" update MUST expire if it has not been approved within 7 days after the update is first proposed.
          1. "Expired" updates MUST be kept offchain and accessible/visible only to multisig signers.
    4. The user MUST be able to see all "Past" updates. A "Past" update is an update that has either been enacted, canceled, or expired.
    5. The user MUST be able to propose new updates on all of the multisigs they are a signer on.
       - Alpen Administrator multisig:
          - Alpen verification key update.
          - Alpen Administrator Signer update.
       - Strata Administrator multisig:
          - Safe Harbor address update.
          - Strata verification key update.
          - Strata Administrator Signer update.
          - Security Council Signer update.
          - Bridge Operator update.
          - "Soft" bridge update.
          - "Hard" bridge update.
       - Strata Sequencer Manager multisig:
          - Strata Sequencer Manager Signer update.
          - Sequencer key update.
       - Security Council multisig:
          - Defcon 1 transaction
          - Defcon 3 transaction
6.  1. The requirements in this section MUST only apply to the Payout Administrator multisig, unless explicitly stated otherwise.
    2. The user MUST be able to see all "Pending" block_payouts transactions, along with the following metadata about the transactions: 
       - How much time is left before the "Pending" block_payouts transaction expires
       - The transaction ID of the "Pending" block_payouts transaction
       - How many block_payouts inputs are part of the transaction, and the specific inputs included.
       - How many approval signatures the "Pending" block_payouts transaction has received out of the total number of required signatures.
       1. A "Pending" block_payouts transaction is a block_payouts transaction that has been proposed but has not yet reached the required quorum of signatures for spending and been broadcast to bitcoin (visible in the public mempool).
       1. All "Pending" block_payouts transactions MUST be kept offchain and accessible/visible only to multisig signers.
       1. The user MUST see an informational message on the relevant transactions if two or more “Pending” block_payouts transactions contain one or more of the same inputs.
          1. The specific inputs that are the same should have an informational icon next to them, with a tooltip that says: “This input is included in multiple Pending transactions.”
       1. The user MUST be able to import and export a raw copy of any "Pending" block_payouts transaction, including any signatures already on the transaction.
       1. The user MUST be able to copy all available signatures for a given block_payouts transaction to their clipboard.
       1. The user MUST be able to paste in the signatures required to approve a given block_payouts transaction.
          1. If any signatures are invalid, then those signatures MUST not be imported and the user MUST receive an error message specifying the signature(s) that are invalid, with a button to copy the full error message to the clipboard.

If one invalid signature:
“Warning: Invalid signature. Please provide a valid signature. 
<invalid signature>”

If multiple invalid signatures:
”Warning: Invalid signatures. Please provide valid signatures. 
<list of invalid signatures>”
       1. The user MUST see a checkmark icon and “Signed” message on any “Pending” transaction that the user has already signed.
       1. The user MUST see a “Sign” button on any "Pending" block_payouts transaction that the user has not already signed.
          1. If the user clicks the “Sign” button, this should trigger a signing flow.
          1. After a user adds the signature that causes the block_payouts transaction to reach quorum, the transaction MUST automatically be broadcast to the bitcoin network using the bitcoin node connected to the application.
       1. A "Pending" block_payouts transaction MUST expire if it has not been confirmed within 4 days after the block_payouts transaction first appears with a signature in the system, or if any of the inputs are spent in a different transaction that appears on bitcoin — whichever comes first.
          1. "Expired" block_payouts transactions MUST be deleted from the backend and removed from the UI.
    3. The user MUST be able to see all "Past" block_payouts transactions, including their confirmation status ("Unconfirmed" or "Confirmed"), block timestamp, and transaction ID. A "Past" block_payouts transaction is a block_payouts transaction that has been broadcast to the bitcoin network.
       1. For each “Unconfirmed” “Past” block_payouts transaction, there MUST be a “Rebroadcast” button and a “Copy to clipboard” button.
       1. Clicking the “Rebroadcast” button MUST attempt to rebroadcast the transaction through the bitcoin node that the application is connected to.
       1. Clicking “Copy to clipboard” MUST copy the raw transaction details to the user’s clipboard, which the user can then paste into a different interface for broadcasting bitcoin transactions.
    4. The user MUST be able to create a new block_payouts transaction by clicking a "Block payouts" button.
       1. The user MUST be able to provide one or more “false claim reports”, from which the application can derive the block_payouts outpoints to be added as inputs to the block_payouts transaction.
          1. The application MUST verify that the false claim proofs contained within the false claim reports are valid, and ignore any claims whose false claim proofs are invalid.
          1. The application MUST ignore any block_payouts outpoints that have already been spent.
       1. The application MUST accept unspent block_payouts outpoints as inputs until the transaction has reached the standard transaction limit according to the latest release of Bitcoin Core, accounting as well for the fee input(s) and one change output to a change address in the user’s Admin Wallet.
       1. The user MUST be able to specify a fee rate in s/VB and in increments of 0.1 s/VB up to 10,000 s/vB.
          1. The input(s) used to pay the mining fee for the transaction MUST come from the user’s connected Admin Wallet, and any change MUST also be sent to the first unused change address in the Admin Wallet.
       1. The user MUST see each of the block_payouts inputs that are included in the transaction, and a numeric total for the number of block_payouts inputs.
          1. The user MUST be able to remove individual block_payouts inputs from the transaction.
       1. The user MUST receive a critical error message if the size of their transaction exceeds standardness limits in the most recent release of Bitcoin Core.
          1. “Your transaction exceeds the size limit, please remove one or more inputs to reduce its size.”
       1. If there are no critical error messages, the user MUST be able to click a “Confirm” button that will create the transaction and add their signature to the transaction.
          1. After the user has created and signed the transaction, it should be added to the "Pending" block_payouts transaction section.
