import { CheckEmeraldIcon, CheckWhiteIcon, UsbTridentIcon } from '@/assets/icons'
import type { ConnectViewState } from '@/domain/connect-wallet/model/hw-wallet-connect.types'

export function ConnectionIcon({ state }: { state: ConnectViewState }) {
	if (state === 'success') {
		return <SuccessIcon tone="emerald" />
	}

	return <UsbTridentIcon width={48} height={48} className="block max-h-full max-w-full object-contain text-[#6b7280]" />
}

export function SuccessIcon({ tone }: { tone: 'emerald' | 'white' }) {
	if (tone === 'emerald') {
		return <CheckEmeraldIcon width={20} height={20} className="block" />
	}
	return <CheckWhiteIcon width={20} height={20} className="block" />
}
