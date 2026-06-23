import type { ActionValidator } from './types'
import { EVEN_PUBKEY_HEX_PATTERN } from './types'

export const validateSequencerKeyUpdate: ActionValidator = ({ data, ctx }) => {
	const key = data.newSequencerKeyHex.trim()
	if (key.length === 0) {
		ctx.addIssue({ code: 'custom', path: ['newSequencerKeyHex'], message: 'New sequencer key is required' })
	} else if (!EVEN_PUBKEY_HEX_PATTERN.test(key)) {
		ctx.addIssue({
			code: 'custom',
			path: ['newSequencerKeyHex'],
			message: 'Sequencer key must be an x-only pubkey hex (32 bytes, 64 hex chars, no 02/03 prefix)',
		})
	}
}
