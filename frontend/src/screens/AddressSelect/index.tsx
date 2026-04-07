import type { ConnectedWallet, WalletAddress } from '@/types'

type Props = {
  wallet: ConnectedWallet
  onSelected: (address: WalletAddress) => void
}

export default function AddressSelect({ wallet, onSelected }: Props) {
  return (
    <div>
      <h1>Select Signing Address</h1>
      <p>{wallet.deviceName}</p>
      <ul>
        {wallet.addresses.map((addr) => (
          <li key={addr.address}>
            <button onClick={() => onSelected(addr)}>
              {addr.address} ({addr.derivationPath})
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
