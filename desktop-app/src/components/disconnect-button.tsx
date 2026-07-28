import { LogOutIcon } from '@/assets/icons'

/**
 * Header "Disconnect" control, shared by every authenticated screen.
 *
 * The markup used to be copied byte for byte into 11 screens, each carrying a
 * red hover — border, fill and text all in the danger family. (Spelling those
 * classes out here would make Tailwind emit them again; it scans comments too.)
 * Ending a session is not an error, so the hover is neutral now (#416), and the
 * single definition keeps the 11 call sites from drifting apart again.
 */
export function DisconnectButton({ onClick }: { onClick: () => void }) {
	return (
		<button
			type="button"
			className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] hover:text-emphasis"
			onClick={onClick}
		>
			<LogOutIcon width={12} height={12} className="shrink-0" />
			Disconnect
		</button>
	)
}
