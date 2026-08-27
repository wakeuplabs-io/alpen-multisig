import type { SVGProps } from 'react'

/** Props for inline SVG icons (no `<img>`). */
export type IconProps = Omit<SVGProps<SVGSVGElement>, 'ref' | 'children'>

export function AlpenMark({ width = 20, height = 19, ...rest }: IconProps) {
	return (
		<svg width={width} height={height} viewBox="0 0 119 108.69" fill="none" aria-hidden {...rest}>
			<polygon
				fill="#0A0A0A"
				points="84.86 0 47.35 0 64.56 39.63 30 39.63 0 108.69 35.75 108.69 65.15 40.99 81.48 78.59 68.39 108.69 105.9 108.69 119 78.59 84.86 0"
			/>
		</svg>
	)
}

export function HwLinkPortsIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<rect x="5" y="7" width="14" height="10" rx="2" stroke="#6b7280" strokeWidth="1.5" />
			<path d="M9 17v3M12 17v3M15 17v3M9 7V4M12 7V4M15 7V4" stroke="#6b7280" strokeWidth="1.5" />
		</svg>
	)
}

/** Classic USB trident (circle / line / square / arrow); use `className` with `text-*` for color. */
export function UsbTridentIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" width={width} height={height} className={className} aria-hidden fill="none" {...rest}>
			<circle cx="12" cy="18.75" r="2.1" fill="currentColor" stroke="none" />
			<g stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
				<path d="M12 16.65V11.25" />
				<path d="M12 11.25 7.9 7.95" />
				<path d="M12 11.25 16.1 7.95" />
				<path d="M12 11.25V6.2" />
				<path d="M9.75 4.25 12 2.5 14.25 4.25" />
			</g>
			<circle cx="6.35" cy="7.1" r="1.35" fill="currentColor" stroke="none" />
			<rect x="15.3" y="5.75" width="2.4" height="2.4" rx="0.4" fill="currentColor" stroke="none" />
		</svg>
	)
}

export function CheckEmeraldIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M5 12l5 5L20 7" stroke="#059669" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

export function CheckWhiteIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M5 12l5 5L20 7" stroke="#fff" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

export function UsbStrokeWhiteIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	const merged = ['text-white', className].filter(Boolean).join(' ')
	return <UsbTridentIcon width={width} height={height} className={merged} {...rest} />
}

export function ShieldCheckMutedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M12 3 4 6v6c0 5 4 8 8 9 4-1 8-4 8-9V6l-8-3Z" stroke="#9ca3af" strokeWidth="1.5" />
			<path d="M9 12l2 2 4-4" stroke="#9ca3af" strokeWidth="1.5" />
		</svg>
	)
}

export function ShieldAccentIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M12 3 4 6v6c0 5 4 8 8 9 4-1 8-4 8-9V6l-8-3Z" stroke="var(--color-accent)" strokeWidth="1.5" />
		</svg>
	)
}

/**
 * Log-out glyph that inherits `currentColor`, so the surrounding button owns
 * the color. Replaces the former muted/red pair that cross-faded on hover —
 * disconnecting is not an error, so it no longer turns red (#416).
 */
export function LogOutIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M10 17H5V7h5M15 12H9M13 8l4 4-4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

export function ClockSessionDefaultIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<circle cx="12" cy="12" r="8" stroke="#6b7280" strokeWidth="1.5" />
			<path d="M12 8v4l3 2" stroke="#6b7280" strokeWidth="1.5" />
		</svg>
	)
}

export function ClockSessionWarningIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<circle cx="12" cy="12" r="8" stroke="currentColor" strokeWidth="1.5" />
			<path d="M12 8v4l3 2" stroke="currentColor" strokeWidth="1.5" />
		</svg>
	)
}

export function UsbSessionDefaultIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	const merged = ['text-[#6b7280]', className].filter(Boolean).join(' ')
	return <UsbTridentIcon width={width} height={height} className={merged} {...rest} />
}

export function UsbSessionWarningIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	const merged = ['text-emphasis-soft', className].filter(Boolean).join(' ')
	return <UsbTridentIcon width={width} height={height} className={merged} {...rest} />
}

export function CheckCircleEmeraldIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<circle cx="12" cy="12" r="8" stroke="#059669" strokeWidth="1.5" />
			<path d="M8 12l2.5 2.5L16 9" stroke="#059669" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

export function ClockIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<circle cx="12" cy="12" r="8" stroke="currentColor" strokeWidth="1.5" />
			<path d="M12 8v4l3 2" stroke="currentColor" strokeWidth="1.5" />
		</svg>
	)
}

export function FileTextMutedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M7 4h7l3 3v13H7z" stroke="#9ca3af" strokeWidth="1.5" />
			<path d="M14 4v4h4M9 12h6M9 16h6" stroke="#9ca3af" strokeWidth="1.5" />
		</svg>
	)
}

export function ChevronRightMutedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M10 6l6 6-6 6" stroke="#9ca3af" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

export function SignaturePenMutedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M4 20l4-1 9-9-3-3-9 9-1 4Z" stroke="#9ca3af" strokeWidth="1.5" strokeLinecap="round" />
			<path d="M13 6l5 5" stroke="#9ca3af" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

export function MinusMutedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M7 12h10" stroke="#9ca3af" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

export function MinusHoverIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M7 12h10" stroke="#374151" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

export function EyeGrayIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M2 12s4-6 10-6 10 6 10 6-4 6-10 6S2 12 2 12Z" stroke="#9ca3af" strokeWidth="1.5" />
			<circle cx="12" cy="12" r="2.5" stroke="#9ca3af" strokeWidth="1.5" />
		</svg>
	)
}

export function PencilWhiteIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M4 20l4-1 9-9-3-3-9 9-1 4Z" stroke="#fff" strokeWidth="1.5" strokeLinecap="round" />
			<path d="M13 6l5 5" stroke="#fff" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

/** Overlapping rectangles; pair with parent `text-*` for stroke color. */
export function CopyClipboardIcon({ width = 18, height = 18, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<rect x="8" y="8" width="12" height="12" rx="2" stroke="currentColor" strokeWidth="1.5" />
			<path d="M6 16H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" strokeWidth="1.5" />
		</svg>
	)
}

/** Circular arrow (undo/restore); pair with parent `text-*` for stroke color. */
export function UndoIcon({ width = 16, height = 16, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 16 16" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path
				d="M2.667 5.333A5.333 5.333 0 1 1 2 8"
				stroke="currentColor"
				strokeWidth="1.333"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			<path
				d="M2.667 2v3.333H6"
				stroke="currentColor"
				strokeWidth="1.333"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

export function SettingsGearIcon({ width = 16, height = 16, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path
				d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			<path
				d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

/** Clipboard with paste arrow. */
export function ClipboardPasteIcon({ width = 18, height = 18, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<rect x="8" y="4" width="10" height="14" rx="2" stroke="currentColor" strokeWidth="1.5" />
			<path d="M11 4V3a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
			<path
				d="M6 8H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-2"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
			/>
			<path d="M14 12l-3 3 3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
			<path d="M11 15h6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

/** Downward arrow into a tray. */
export function DownloadIcon({ width = 18, height = 18, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path
				d="M12 3v13M7 11l5 5 5-5"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			<path d="M4 20h16" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

/** Folder with up arrow — load proposal from JSON file. */
export function ImportJsonIcon({ width = 18, height = 18, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path
				d="M3 7a2 2 0 0 1 2-2h3l2 2h9a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinejoin="round"
			/>
			<path
				d="M12 11v5M9.5 13.5 12 11l2.5 2.5"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

/** Trash can; pair with parent `text-*` for stroke color. */
export function TrashIcon({ width = 16, height = 16, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 16 16" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path
				d="M2 4h12M5.333 4V2.667A.667.667 0 0 1 6 2h4a.667.667 0 0 1 .667.667V4M6.667 7.333v4M9.333 7.333v4M2.667 4l.666 8.667a.667.667 0 0 0 .667.666h8a.667.667 0 0 0 .667-.666L13.333 4"
				stroke="currentColor"
				strokeWidth="1.333"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

/** Wallet glyph (body + top seam + clasp); pair with parent `text-*` for color. */
export function WalletIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<rect x="3" y="6" width="18" height="13" rx="2.5" stroke="currentColor" strokeWidth="1.6" />
			<path d="M3 9.5h18" stroke="currentColor" strokeWidth="1.6" />
			<circle cx="16.5" cy="13.5" r="1.25" fill="currentColor" />
		</svg>
	)
}

/** Hourglass (slow fee preset); pair with parent `text-*` for color. */
export function HourglassIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
				<path d="M5 2h14M5 22h14" />
				<path d="M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22" />
				<path d="M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2" />
			</g>
		</svg>
	)
}

/** Speed gauge with needle (medium fee preset); pair with parent `text-*` for color. */
export function GaugeIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
				<path d="m12 14 4-4" />
				<path d="M3.34 19a10 10 0 1 1 17.32 0" />
			</g>
		</svg>
	)
}

/** Lightning bolt (fast fee preset); pair with parent `text-*` for color. */
export function BoltIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path
				d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"
				stroke="currentColor"
				strokeWidth="1.75"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

/** Horizontal sliders (custom fee rate); pair with parent `text-*` for color. */
export function SlidersIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
				<path d="M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3" />
				<path d="M14 2v4M8 10v4M16 18v4" />
			</g>
		</svg>
	)
}

/** Plus sign for stepper buttons; pair with parent `text-*` for color. */
export function PlusIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path d="M12 5v14M5 12h14" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

/** Minus sign for stepper buttons; pair with parent `text-*` for color. */
export function MinusIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path d="M5 12h14" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

/** Close (X); pair with parent `text-*` for color. */
export function CloseIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path d="M6 6l12 12M18 6L6 18" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

/** Chevron pointing down; pair with parent `text-*` for color. */
export function ChevronDownIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path d="M6 10l6 6 6-6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
		</svg>
	)
}

/** Warning triangle with exclamation mark; pair with parent `text-*` for color. */
export function AlertTriangleIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
				<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
				<path d="M12 9v4M12 17h.01" />
			</g>
		</svg>
	)
}

/** Shield outline with diamond — authority: alpen admin. */
export function AuthorityShieldIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path d="M12 3 4 6v6c0 5 4 8 8 9 4-1 8-4 8-9V6l-8-3Z" stroke="currentColor" strokeWidth="1.5" />
			<path d="M12 9.5 14.5 12 12 14.5 9.5 12Z" stroke="currentColor" strokeWidth="1.25" />
		</svg>
	)
}

/** Paper-plane send icon. */
export function SendIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<path
				d="M22 2 11 13M22 2l-7 20-4-9-9-4 20-7Z"
				stroke="currentColor"
				strokeWidth="1.75"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

/** Stacked layers — authority: strata admin. */
export function AuthorityLayersIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
				<path d="m2 12 10 5 10-5" />
				<path d="m2 17 10 5 10-5" />
				<path d="m12 2 10 5-10 5L2 7z" />
			</g>
		</svg>
	)
}

/** Grid/server — authority: sequencer manager. */
export function AuthorityServerIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
				<rect x="3" y="3" width="18" height="6" rx="1.5" />
				<rect x="3" y="15" width="18" height="6" rx="1.5" />
				<path d="M7 6h.01M7 18h.01" />
			</g>
		</svg>
	)
}

/**
 * Octagonal halt sign — authority: security council.
 *
 * Deliberately shares no silhouette with the other three authority icons: the council's one action
 * is the emergency lever that halts the bridge, and an authority that looks like another authority
 * is a signer-safety problem, not a cosmetic one.
 */
export function AuthorityHaltIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<g stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
				<path d="M8.5 3.5h7l5 5v7l-5 5h-7l-5-5v-7z" />
				<path d="M8.5 12h7" />
			</g>
		</svg>
	)
}

/** Neutral placeholder for an authority with no icon of its own — never another authority's glyph. */
export function AuthorityUnknownIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} className={className} aria-hidden {...rest}>
			<circle cx="12" cy="12" r="8.5" stroke="currentColor" strokeWidth="1.5" strokeDasharray="3 2.5" />
		</svg>
	)
}
