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

export function ShieldPurpleIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M12 3 4 6v6c0 5 4 8 8 9 4-1 8-4 8-9V6l-8-3Z" stroke="#7c6fcd" strokeWidth="1.5" />
		</svg>
	)
}

export function LogOutMutedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M10 17H5V7h5M15 12H9M13 8l4 4-4 4" stroke="#6b7280" strokeWidth="1.5" strokeLinecap="round" />
		</svg>
	)
}

export function LogOutRedIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<path d="M10 17H5V7h5M15 12H9M13 8l4 4-4 4" stroke="#b91c1c" strokeWidth="1.5" strokeLinecap="round" />
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
			<circle cx="12" cy="12" r="8" stroke="#d97706" strokeWidth="1.5" />
			<path d="M12 8v4l3 2" stroke="#d97706" strokeWidth="1.5" />
		</svg>
	)
}

export function UsbSessionDefaultIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	const merged = ['text-[#6b7280]', className].filter(Boolean).join(' ')
	return <UsbTridentIcon width={width} height={height} className={merged} {...rest} />
}

export function UsbSessionWarningIcon({ width = 24, height = 24, className, ...rest }: IconProps) {
	const merged = ['text-[#d97706]', className].filter(Boolean).join(' ')
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

export function ClockAmberIcon({ width = 24, height = 24, ...rest }: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" width={width} height={height} aria-hidden {...rest}>
			<circle cx="12" cy="12" r="8" stroke="#d97706" strokeWidth="1.5" />
			<path d="M12 8v4l3 2" stroke="#d97706" strokeWidth="1.5" />
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
			<circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.5" />
			<path
				d="M12 2v2M12 20v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M2 12h2M20 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
				stroke="currentColor"
				strokeWidth="1.5"
				strokeLinecap="round"
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
