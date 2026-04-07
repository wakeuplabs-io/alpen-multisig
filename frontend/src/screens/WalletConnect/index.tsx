import { useWallet } from '@/hooks/useWallet'

type Props = {
  onConnected: () => void
}

export default function WalletConnect({ onConnected }: Props) {
  const { state, handleConnect } = useWallet()

  return (
    <div>
      <h1>Connect Hardware Wallet</h1>
      {state.status === 'error' && <p>{state.message}</p>}
      <button
        onClick={async () => {
          await handleConnect()
          onConnected()
        }}
        disabled={state.status === 'connecting'}
      >
        {state.status === 'connecting' ? 'Connecting…' : 'Connect Wallet'}
      </button>
    </div>
  )
}
