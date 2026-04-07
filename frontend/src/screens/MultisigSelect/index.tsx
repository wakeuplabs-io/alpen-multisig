import type { Authority, WalletAddress } from '@/types'
import { AUTHORITY_LABELS } from '@/types'

const AUTHORITIES: Authority[] = [
  'alpen_admin',
  'strata_admin',
  'sequencer_manager',
  'security_council',
  'payout_admin',
]

type Props = {
  selectedAddress: WalletAddress
  onSelected: (authority: Authority) => void
}

export default function MultisigSelect({ selectedAddress, onSelected }: Props) {
  return (
    <div>
      <h1>Select Multisig Authority</h1>
      <p>Signing as: {selectedAddress.address}</p>
      <ul>
        {AUTHORITIES.map((authority) => (
          <li key={authority}>
            <button onClick={() => onSelected(authority)}>
              {AUTHORITY_LABELS[authority]}
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
