/**
 * Real-app smoke: mnemonic path through authority selection and session auth to /proposals.
 * Requires stack (bitcoind, ASM, orchestrator, Postgres) and .env — see README.md.
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

describe('Strata Multisig wallet smoke', () => {
	it('connects with mnemonic and reaches /proposals', async () => {
		await loginMnemonicToProposals(DEMO_MNEMONIC)
	})
})
