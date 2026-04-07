import { useState } from 'react'
import type { Authority, Session, WalletAddress, ConnectedWallet } from '@/types'
import WalletConnect from '@/screens/WalletConnect'
import AddressSelect from '@/screens/AddressSelect'
import MultisigSelect from '@/screens/MultisigSelect'
import Dashboard from '@/screens/Dashboard'

// Navigation flow per PRD:
// wallet connect → address select → multisig select → nonce auth → dashboard
type AppStep =
  | { step: 'wallet-connect' }
  | { step: 'address-select'; wallet: ConnectedWallet }
  | { step: 'multisig-select'; wallet: ConnectedWallet; address: WalletAddress }
  | { step: 'authenticating'; wallet: ConnectedWallet; address: WalletAddress; authority: Authority }
  | { step: 'dashboard'; session: Session }

export default function App() {
  const [nav, setNav] = useState<AppStep>({ step: 'wallet-connect' })

  // TODO: replace stubs with real wallet/auth integration
  if (nav.step === 'wallet-connect') {
    return (
      <WalletConnect
        onConnected={() =>
          // Stub: in real flow, wallet is passed from useWallet
          setNav({ step: 'address-select', wallet: { deviceName: 'Ledger', addresses: [] } })
        }
      />
    )
  }

  if (nav.step === 'address-select') {
    return (
      <AddressSelect
        wallet={nav.wallet}
        onSelected={(address) =>
          setNav({ step: 'multisig-select', wallet: nav.wallet, address })
        }
      />
    )
  }

  if (nav.step === 'multisig-select') {
    return (
      <MultisigSelect
        selectedAddress={nav.address}
        onSelected={(authority) =>
          setNav({ step: 'authenticating', wallet: nav.wallet, address: nav.address, authority })
        }
      />
    )
  }

  if (nav.step === 'authenticating') {
    // TODO: trigger nonce auth flow, then transition to dashboard
    return <p>Authenticating…</p>
  }

  if (nav.step === 'dashboard') {
    return <Dashboard session={nav.session} />
  }

  return null
}
