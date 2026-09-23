import { AlertTriangleIcon } from '@/assets/icons'
import { DEFCON_COPY, type DefconLevel } from '@/lib/defcon-copy'

type Props = {
	level: DefconLevel
	/**
	 * `create` is the roomier block the form and its preview carry; `sign` is the tighter one on
	 * the screen where the signer commits, and it reads the shorter of the two paragraphs.
	 */
	variant?: 'create' | 'sign'
}

/**
 * The destructive block both Defcon levers carry. Presentational: it takes a level and renders
 * that level's own words, which is the whole point — the same paragraph used to be written out in
 * three places, and a second lever would have made three places disagree.
 */
export function DefconCallout({ level, variant = 'create' }: Props) {
	const copy = DEFCON_COPY[level]
	const isSign = variant === 'sign'

	return (
		<div className={`border border-danger-border bg-danger-surface ${isSign ? 'rounded-lg p-3.5' : 'rounded-xl p-4'}`}>
			<p className="m-0 flex items-center gap-2 text-body font-semibold text-danger-deep">
				<AlertTriangleIcon width={isSign ? 15 : 16} height={isSign ? 15 : 16} className="shrink-0 text-danger" />
				{copy.calloutTitle}
			</p>
			<p className={`m-0 mt-2 ${isSign ? 'text-label' : 'text-body'} text-danger-deep`}>
				{isSign ? copy.signCalloutBody : copy.calloutBody}
			</p>
		</div>
	)
}
