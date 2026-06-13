import { QRCodeSVG } from 'qrcode.react'

export type QrCodeProps = {
	/** The string encoded in the QR (for a receive address, the bare address). */
	value: string
	/** Pixel size of the square QR. Defaults to 132. */
	size?: number
	/** Accessible name for assistive tech. */
	ariaLabel?: string
}

/**
 * Render a string as an accessible inline-SVG QR code. Thin wrapper over
 * `qrcode.react` so the rest of the app stays decoupled from the QR library.
 * High error-correction ('M') keeps the code scannable with a little padding.
 */
export function QrCode({ value, size = 132, ariaLabel }: QrCodeProps) {
	return (
		<QRCodeSVG
			value={value}
			size={size}
			level="M"
			marginSize={2}
			bgColor="#ffffff"
			fgColor="#111827"
			role="img"
			aria-label={ariaLabel ?? 'QR code'}
		/>
	)
}
