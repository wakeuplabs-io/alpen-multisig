import { formatBtcFromSats } from './format-btc-from-sats'

export type Denomination = 'BTC' | 'sats'

export type DenominatedBalance = {
	amount: string
	unit: Denomination
}

export function toggleDenomination(current: Denomination): Denomination {
	return current === 'BTC' ? 'sats' : 'BTC'
}

export function denominateSats(sats: number, denomination: Denomination): DenominatedBalance {
	if (denomination === 'sats') {
		return { amount: Number.isFinite(sats) ? Math.trunc(sats).toLocaleString('en-US') : '—', unit: 'sats' }
	}
	return { amount: formatBtcFromSats(sats), unit: 'BTC' }
}

export function formatDenominatedBalance(balance: DenominatedBalance): string {
	return `${balance.amount} ${balance.unit}`
}
