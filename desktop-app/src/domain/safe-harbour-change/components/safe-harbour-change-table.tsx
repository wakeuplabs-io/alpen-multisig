import type { SafeHarbourChange, SafeHarbourDestination } from '../model/build-safe-harbour-change'

type Props = {
	change: SafeHarbourChange
}

/**
 * Where the bridge sweeps to, and where it would sweep to instead.
 *
 * Two columns when there is a comparison to make and one when there is not — an enacted rotation
 * has no recoverable "before", and printing the live destination on both sides would read as a
 * rotation that changed nothing.
 *
 * Both forms of each destination are shown, address above descriptor. The descriptor is the value
 * the signer's device displays, so it is the one they can actually check; the address is the one
 * they recognise.
 */
export function SafeHarbourChangeTable({ change }: Props) {
	if (change.from === null) {
		return (
			<div className="px-4 py-3" data-testid="e2e-safe-harbour-change">
				<p className="m-0 mb-1.5 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">Destination</p>
				<Destination destination={change.to} />
			</div>
		)
	}

	return (
		<table
			className="w-full table-fixed border-collapse"
			data-testid="e2e-safe-harbour-change"
			aria-label="Sweep destination change"
		>
			<thead>
				<tr className="border-b border-[#f3f4f6] bg-[#f9fafb]">
					<th
						scope="col"
						className="w-1/2 px-4 py-2.5 text-left text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]"
					>
						Current
					</th>
					<th
						scope="col"
						className="w-1/2 border-l border-[#f3f4f6] px-4 py-2.5 text-left text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]"
					>
						Proposed
					</th>
				</tr>
			</thead>
			<tbody>
				<tr>
					<td className="px-4 py-2.5 align-top">
						<Destination destination={change.from} />
					</td>
					<td className="border-l border-[#f3f4f6] px-4 py-2.5 align-top">
						<Destination destination={change.to} />
					</td>
				</tr>
			</tbody>
		</table>
	)
}

function Destination({ destination }: { destination: SafeHarbourDestination }) {
	return (
		<span className="flex flex-col gap-1">
			{destination.address.length > 0 && (
				<span className="break-all font-mono text-mono-sm leading-relaxed text-[#374151]">{destination.address}</span>
			)}
			<span className="break-all font-mono text-mono-sm leading-relaxed text-[#9ca3af]">{destination.addressHex}</span>
		</span>
	)
}
