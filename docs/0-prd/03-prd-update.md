```
// https://alpenlabs.notion.site/PRD-Strata-multisig-app-2c4901ba000f805eb6c4d0d62bb4e74f
```

# PRD: Strata multisig app

**Background**

The Strata protocol specifies several multisigs for administrative functions, namely the Strata Administrator, the Strata Sequencer Manager, the Strata Security Council, and the Payout Administrator. Additionally, the Alpen protocol specifies an administrative multisig, the Alpen Administrator. This document specifies requirements for a minimal, cross-platform desktop application ("the application") that enables easy and secure management of these multisigs by their respective signers.

**Additional references and resources**

- [Strata Multisig Backend - Design Guidelines & Architectural Notes](https://www.notion.so/317901ba000f8064bfc4cd7433e1261f?pvs=21)
- [SPS-50: L1 transaction header and interpretation](https://www.notion.so/317901ba000f800c8f5ee39d810b159a?pvs=21)
- [SPS-51: Generic simple envelope format](https://www.notion.so/317901ba000f809caeaadab8951ddf17?pvs=21)
- [SPS-65: Strata administration subprotocol (transaction processing subsection)](https://www.notion.so/317901ba000f80bf8d96eb5ef0667772?pvs=21)

**Roles**

Bridge Operators: a user whose public key was included in the canonical list of bridge operators at the time of a Strata bridge deposit.

Alpen Administrator Signer: a user whose public key is listed in the Alpen consensus protocol as one of the signer keys on the Alpen Administrator multisig.

Strata Administrator Signer: a user whose public key is listed in the Strata consensus protocol as one of the signer keys on the Strata Administrator multisig.

Strata Sequencer Manager Signer: a user whose public key is listed in the Strata consensus protocol as one of the signer keys on the Strata Sequencer Manager multisig.

Strata Security Council Signer: a user whose public key is listed in the Strata consensus protocol as one of the signer keys on the Strata Security Council multisig.

Payout Administrator Signer: a user who public key is included as a spender of the `block_payout` transaction in the Strata bridge bitcoin script.

Note: Where the term "user" is used without further specification, the requirement applies to all roles described above.

**Requirements**

1.  1. The user MUST be able to run the application locally on their desktop on an up-to-date version of the latest Long Term Support (LTS) release of the Debian Linux, Mac, or Windows operating system, using computer hardware with a minimum of 8 GB RAM, 2c4t CPU, 1 TB SSD, and 20 Mbps internet.
    2. Builds of the application MUST be [reproducible](https://reproducible-builds.org/docs/definition/).
    3. The user SHOULD be able to cryptographically verify that the application binary they are running was published and approved by multiple employees of Alpen Labs.
    4. The user MUST be able to install or run the application with either a single terminal command or double-click on an application icon.
       1. The installation of dependencies MUST take no more than one additional command or click, if any additional steps are required at all for installing dependencies.
       2. Any clicks required to approve administrative privileges for installing the application can be disregarded for this requirement.
2.  1. The application MUST support accessing read/write functionality for bitcoin and Strata using either a trusted RPC endpoint or by connecting to a Strata node running locally on the same desktop.
       1. The user MUST be able to select a trusted RPC endpoint run on the [https://stratabtc.org](https://stratabtc.org/) domain for accessing read/write functionality for bitcoin and Strata, or enter their own custom RPC URL.
       2. The default connection method MUST be connecting to a node running locally on the same desktop.
          1. If no local node is detected, then the user MUST be prompted to either turn on their local node or switch the connection method to use a trusted RPC endpoint.
       3. If the user is running a local Strata node, the application SHOULD be able to access read/write functionality for bitcoin and Strata using that node with no additional effort from the user.
3.  1. The user MUST be able to select the multisig they want to interact with. The supported multisigs MUST be:
       1. Alpen Administrator multisig. MUST be usable exclusively by all Alpen Administrator Signers.
       2. Strata Administrator multisig. MUST be usable exclusively by all Strata Administrator Signers
       3. Strata Sequencer Manager multisig. MUST be usable exclusively by all Strata Sequencer Manager Signers.
       4. Strata Security Council multisig. MUST be usable exclusively by all Strata Security Council Signers.
       5. Payout Administrator multisig. MUST be usable exclusively by all Payout Administrator signers.
    2. 1. The user MUST be able to connect a supported hardware wallet and see a list of addresses to choose from.
       1. Supported hardware wallets MUST include all hardware wallets currently supported by [HWI](https://github.com/bitcoin-core/HWI/blob/master/docs/devices/index.rst) using the following features:
          - Taproot inputs,
          - Message signing,
          - Display on device screen, and
          - Are otherwise compatible with SPS-65 updates.
       1. Admin ID:
          1. If the user selected the Payout Administrator in the previous step, then the application MUST use the user’s connected hardware wallet to generate an “Admin ID” represented by a P2TR bitcoin address generated using the [BIP-86-compliant](https://bips.dev/86/) `m/86'/0'/73'/0/0` derivation path, where `73'` is the hardened account used to generate the Admin ID, and `/0/0` are the change and address indexes, respectively. The Admin ID MUST be used for authenticating with the multisig app backend and signing all Payout Administrator transactions, as described in Requirement 6.
          2. If the user selected any multisig other than the Payout Administrator in the previous step, then the application MUST use the user’s connected hardware wallet to generate an “Admin ID” represented by a P2WPKH bitcoin address generated using the [BIP-84-compliant](https://bips.dev/84/) `m/84'/0'/73'/0/0` derivation path of the connected hardware wallet, where `73'` is the hardened account used to generate the Admin ID, and `0` is the address index. The Admin ID MUST be used for authenticating with the multisig app backend and signing all admin subprotocol update-related messages. For the avoidance of doubt, the Admin ID MUST NOT be used to sign any bitcoin transactions.
       1. Admin Wallet: The application MUST use the user’s connected hardware wallet to generate an “Admin Wallet” using the [BIP-86-compliant](https://bips.dev/86/) `m/86'/0'/73'/n/n` derivation path, where `73'` is the hardened account used to generate the Admin Wallet, and `/n/n` are the change and address indexes, respectively.I
       1. The user MUST be able to clearly read and understand each message they are signing on their hardware wallet screen, to be able to visually verify that the message they are signing matches what they are expecting based on what they are seeing in the application UI.
    3. After connecting their Admin ID and Admin Wallet, the user MUST sign a nonce with the private key of their Admin ID to gain access to the UI for the selected multisig.
       1. The user MUST only be given access to the UI for the selected multisig if the address that they have signed the nonce for is on the canonical list of multisig signers.
       2. If the user produces an invalid signature, then the user SHOULD be shown an error message saying so.
       3. If the user produces a valid signature, but the connected Admin ID is not on the canonical list of signers on the selected multisig, then the user SHOULD be shown an error message saying so.
    4. After gaining access to view the selected multisig, the user MUST be able to close the selected multisig UI and go back to the multisig selection screen.
    5. After gaining access to view the selected multisig, the user MUST be able to disconnect the selected address and go back to the wallet connection screen.
4.  1. After logging in to the application, the user MUST be able to see their Admin ID and copy it to the clipboard.
    2. The user MUST be able to use the application to view their Admin ID on their hardware wallet screen to verify that the Admin ID they see in the UI was actually derived from the seed phrase loaded in their connected hardware wallet.
    3. The user MUST be able to manage their Admin Wallet using the application, including:
       1. Balance: The user MUST be able to see the total BTC balance of the wallet (net of unconfirmed send and receive transactions) and the net balance of all unconfirmed send and receive transactions.
       2. Addresses: The user MUST be able to see each address in the wallet that holds a balance along with the current balance of each address (net of unconfirmed transactions).
       3. Transactions: The user MUST be able to see each unconfirmed transaction sent from the Admin Wallet and have the ability to bump the fee
       4. Receive: see the first unused address in the wallet address index, in both text and QR code formats.
          1. Clicking on the address text or QR code MUST copy the address to the user’s clipboard.
          2. The user MUST be able to click a button to view their receive address on their hardware wallet screen to verify that the address they see in the UI was actually derived from the seed phrase loaded in their connected hardware wallet.
          3. Each address SHOULD be “one-time use” i.e. after the user has received BTC in a given address, the app MUST automatically rotate the address on the “receive” screen to show a different, unused address.
       5. Send BTC:
          1. Send to: The user MUST be able to enter any standard bitcoin address to send to i.e. P2PK, P2PKH, P2SH, P2WPKH, P2WSH, P2TR. If the user enters an output type that is not standard or consensus valid, then the user MUST be shown a critical error message: "Destination must be a bitcoin address." If the user enters a destination address that is not a bitcoin address on the correct network (e.g. the user enters a testnet bitcoin address when the wallet is currently connected to bitcoin mainnet) then the user MUST be shown a critical error message: "Destination must be a [correct network] bitcoin address." where [correct network] says `mainnet` or `testnet`, depending on which network the wallet is connected to.
          2. Amount: The user MUST be able to specify any valid amount of BTC to send. A “valid amount” is any amount where `amount` ≤ `wallet balance - (fee rate s/vB * transaction size vB)` , or click a “Max” button to automatically enter the maximum allowed amount. If the user enters a send amount that results in `send amount + mining fee > wallet balance` , then the user MUST be shown a critical error message: “Insufficient funds”.
          3. The user MUST be able to manually specify the fee rate of their send transaction in s/VB, in increments of 0.1 s/VB up to 10,000 s/vB. The default fee rate shown MUST be the “next block” fee rate provided by the Bitcoin Core node that the application is connected to.
          4. Change from the send transaction (if any) MUST be sent to the first unused address in the wallet’s change index.
          5. There MUST be a "Confirm" button that is disabled by default, and is only enabled when all send form text entry fields are correctly filled.
             1. Clicking the "Confirm" button when it is enabled MUST send a bitcoin transaction to the user’s connected hardware wallet; if the user confirms the transaction on their hardware wallet, then the transaction MUST be sent to the bitcoin network for confirmation and the user MUST be shown the transaction ID; if the user rejects the transaction in their hardware wallet then nothing should happen in the UI and the user can either click “Confirm” to try again or else back out of the “send” screen.
5.  1. The requirements in this section MUST only apply to the following multisigs, unless explicitly stated otherwise:
       - Alpen Administrator multisig
       - Strata Administrator multisig
       - Strata Sequencer Manager multisig
       - Strata Security Council multisig
    2. The user MUST be able to see all "Approved" updates and how many cancellation signatures each "Approved" update has received (if any). An "Approved" update is an update that has reached the required quorum of approval signatures and has been confirmed onchain, but has not yet been enacted.
       1. The user MUST be able to cancel any "Approved" update.
          1. The user MUST be able to copy all available cancellation signatures for a given update to their clipboard.
          2. The user MUST be able to create a cancellation transaction for a given "Approved" update, paste in the quorum of signatures required to cancel the update, and broadcast the cancellation transaction to bitcoin for confirmation either using the application's bitcoin RPC or by copying the raw transaction to the clipboard and broadcasting the transaction through any other bitcoin RPC.
          3. Canceled updates MUST be kept offchain and accessible/visible only to multisig signers.
       2. For the avoidance of doubt, this subsection does not apply to the following multisigs, because they do not produce update types that have an "Approved" or "Canceled" state:
          - Strata Sequencer Manager multisig
          - Strata Security Council multisig
    3. The user MUST be able to see all "Pending" updates, including how much time is left before the "Pending" update expires and how many approval signatures the "Pending" update has received (if any) out of the total number of required signatures. A "Pending" update is an update that has been proposed but has not yet reached the required quorum of signatures for approval and been confirmed on bitcoin.
       1. All "Pending" updates MUST be kept offchain and accessible/visible only to multisig signers.
       2. The user MUST be able produce an approval signature for any "Pending" update.
          1. The user MUST be able to copy all available approval signatures for a given "Pending" update to their clipboard.
          2. The user MUST be able to create an approval transaction for a given "Pending" update, paste in the quorum of signatures required to approve the update, and broadcast the approval transaction to bitcoin for confirmation either using the application's bitcoin RPC or by copying the raw transaction to the clipboard and broadcasting the transaction through any other bitcoin RPC. This flow SHOULD have a UI/UX similar to the “send” screen in the “wallet” section.
          3. The user whose signature causes the update transaction to reach its quorum SHOULD be given the option of creating, signing, and broadcasting the bitcoin transaction necessary for the update to be confirmed on bitcoin, or declining to do so.
             1. "Pending" updates that have reached quorum but have not been confirmed yet MUST have a "Send" button that, when clicked, enable the user to create, sign, and broadcast the bitcoin transactions necessary for the update to be confirmed on bitcoin, with a UI/UX similar to the “send” screen in the “wallet” section.
       3. A "Pending" update MUST expire if it has not been approved within `7` days after the update is first proposed.
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
    2. The user MUST be able to see all "Pending" `block_payout` transactions, including how much time is left before the "Pending" `block_payout` transaction expires, the transaction ID of the "Pending" `block_payout` transaction, and how many approval signatures the "Pending" `block_payout` transaction has received (if any) out of the total number of required signatures. A "Pending" `block_payout` transaction is `block_payout` transaction that has been proposed but has not yet reached the required quorum of signatures for spending.
       1. All "Pending" `block_payout` transactions MUST be kept offchain and accessible/visible only to multisig signers.
       2. The user MUST be able to import and export a raw copy of any "Pending" `block_payout` transaction.
       3. The user MUST be able produce a spend signature for any "Pending" `block_payout` transaction.
          1. The user MUST be able to copy all available spend signatures for a given `block_payout` transaction to their clipboard.
          2. The user MUST be able to paste in the signatures required to approve a given `block_payout` transaction, and broadcast the signed `block_payout` transaction to bitcoin for confirmation either using the application's bitcoin RPC or by copying the raw signed transaction to the clipboard and broadcasting the transaction through any other bitcoin RPC.
          3. The user whose signature causes the `block_payout` transaction to reach quorum SHOULD be given the option of either broadcasting the transaction to be confirmed on bitcoin, or declining to do so.
             1. "Pending" `block_payout` transactions that have reached quorum but have not been confirmed yet MUST have a "Send" button that, when clicked, broadcasts the transaction to be confirmed on bitcoin.
       4. A "Pending" `block_payout` transaction MUST expire if it has not been spent within `7` days after the `block_payout` transaction first appears with a signature in the system.
          1. "Expired" `block_payout` transactions MUST be deleted from the backend and removed from the UI.
    3. The user MUST be able to see all "Past" `block_payout` transactions, including their confirmation status ("Unconfirmed" or "Confirmed"), block timestamp, and transaction ID. A "Past" `block_payout` transaction is a `block_payout` transaction that has been broadcast to the bitcoin network.
    4. The user MUST be able to manually create a "Pending" `block_payout` transaction by providing `block_payout` inputs for the transaction, specifying a fee rate in s/VB and in increments of 0.1 s/VB up to 10,000 s/vB, then adding their signature to the transaction.
       1. The input(s) used to pay the mining fee for the transaction MUST come from the user’s connected Admin Wallet, and any change MUST also be sent to the first unused change address in the Admin Wallet.
       2. The user MUST receive a critical error message if the size of their transaction exceeds standardness limits in the most recent release of Bitcoin Core.
    5. The user MUST be able to create a new `block_payout` transaction by clicking a "Block payouts" button.
       1. This transaction MUST automatically create a `block_payout` transaction that includes as many unspent `block_payout` inputs as will fit into a standard transaction, accounting as well for the signatures that need to be added to spend the inputs and the fee input(s) and change output.
       2. The user MUST see how many `block_payout` inputs are included in the transaction.
       3. The user MUST be able to add their signature to the new `block_payout` transaction, which will then add the transaction to the "Pending" `block_payout` transaction section.
       4. If a user clicks the "Block payouts" button before the most recently-created "Pending" `block_payout` transaction has been confirmed, then the new `block_payout` transaction generated MUST be the same as the previous (most recently-created) "Pending" `block_payout` transaction.
