import { useState } from 'react'

type Props = {
	loading: boolean
	onSubmit: (code: string) => void
}

const CODE_LENGTH = 6

/**
 * The code a Trezor Safe 7 shows the first time the app connects to it in a run. The device
 * displays it as two groups ("123 456"); anything but the digits is dropped as it is typed.
 */
export function PairingCodeForm({ loading, onSubmit }: Props) {
	const [code, setCode] = useState('')
	const isComplete = code.length === CODE_LENGTH

	return (
		<form
			className="mt-5 rounded-lg border border-accent-border bg-bg-surface px-3.5 py-3 text-body-sm"
			onSubmit={(event) => {
				event.preventDefault()
				if (isComplete) onSubmit(code)
			}}
		>
			<label htmlFor="pairing-code" className="font-medium text-[#0a0a0a]">
				Enter the code shown on your Trezor
			</label>
			<p className="m-0 mt-0.5 text-label text-[#6b7280]">
				Your Trezor asks to pair with this computer before it connects. Type the 6-digit code on its screen.
			</p>
			<div className="mt-2 flex items-center gap-2">
				<input
					id="pairing-code"
					data-testid="e2e-trezor-pairing-code"
					className="w-full rounded-md border border-[#d1d5db] bg-white px-3 py-2 font-mono text-body tracking-[0.3em] text-[#111827] outline-none focus:border-[#9ca3af]"
					inputMode="numeric"
					autoComplete="one-time-code"
					autoFocus
					placeholder="000000"
					value={code}
					onChange={(event) => setCode(event.target.value.replace(/\D/g, '').slice(0, CODE_LENGTH))}
					disabled={loading}
				/>
				<button
					type="submit"
					data-testid="e2e-trezor-pairing-submit"
					className="rounded-md border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-label font-medium text-white transition hover:bg-[#2a2a2a] disabled:cursor-not-allowed disabled:opacity-60"
					disabled={loading || !isComplete}
				>
					Pair
				</button>
			</div>
		</form>
	)
}
