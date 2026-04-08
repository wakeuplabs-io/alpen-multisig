import { useState } from 'react'
import type { ConnectedWallet, WalletAddress } from '@/types'

type WalletState =
  | { status: 'disconnected' }
  | { status: 'connecting' }
  | { status: 'connected'; wallet: ConnectedWallet; selectedAddress: WalletAddress | null }
  | { status: 'error'; message: string }

export function useWallet() {
  const [state, setState] = useState<WalletState>({ status: 'disconnected' })

  async function handleConnect() {
    setState({ status: 'connecting' })
    // TODO: invoke Tauri HWI command to detect and connect hardware wallet
  }

  function handleSelectAddress(address: WalletAddress) {
    if (state.status !== 'connected') return
    setState({ ...state, selectedAddress: address })
  }

  function handleDisconnect() {
    setState({ status: 'disconnected' })
  }

  return { state, handleConnect, handleSelectAddress, handleDisconnect }
}
