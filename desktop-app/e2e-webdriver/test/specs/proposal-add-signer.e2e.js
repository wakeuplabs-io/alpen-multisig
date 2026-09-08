/**
 * Creates a Strata Administrator signer-update proposal adding one compressed pubkey.
 * Requires full stack + same mnemonic session as wallet-smoke — see README.md.
 */
import { DEMO_MNEMONIC, loginMnemonicToProposals } from '../helpers/login-mnemonic.mjs'

const CANDIDATE_SIGNERS = [
	'03dd6d7dbd51e832af4c8eba8a7bf08ae616054b3e2e2e0823a8167c4def1e427c',
	'02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5',
	'0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798',
]

describe('Strata Multisig proposal — add signer', () => {
	it('drafts signer update, previews, and collects creator signature', async function () {
		this.timeout(180000)
		await loginMnemonicToProposals(DEMO_MNEMONIC)

		// Do not use browser.url('/proposals/create'): Tauri's asset protocol resolves paths as
		// static files (no SPA fallback). In-app navigation matches production WebView behavior.
		const createNav = await $('button[data-testid="e2e-dashboard-create-proposal"]')
		await createNav.waitForClickable({ timeout: 60000 })
		await createNav.click()

		await $('//h1[contains(.,"Create")]').waitForDisplayed({ timeout: 60000 })
		await $('//h1[contains(.,"proposal")]').waitForDisplayed({ timeout: 10000 })

		// Default action is signer_update; ensure card selected (idempotent click).
		await $('//button[.//p[contains(text(),"Signer update")]]').click()

		const proposalTitle = `E2E add signer ${Date.now()}`
		const title = await $('input[data-testid="e2e-create-proposal-title"]')
		await title.waitForDisplayed({ timeout: 30000 })
		await title.setValue(proposalTitle)

		const formText = await $('body').getText()
		const newSigner = CANDIDATE_SIGNERS.find((candidate) => !formText.includes(candidate))
		expect(newSigner).toBeDefined()
		const pubkeyIn = await $('input[data-testid="e2e-new-signer-pubkey-input"]')
		await pubkeyIn.waitForDisplayed({ timeout: 60000 })
		await pubkeyIn.setValue(newSigner)
		await $('button[data-testid="e2e-new-signer-add-button"]').click()

		await browser.waitUntil(
			async () => (await $('span[data-testid="e2e-added-signer-value"]').getText()) === newSigner,
			{ timeout: 15000, timeoutMsg: 'added signer pubkey should appear in list' },
		)

		const previewBtn = await $('button[data-testid="e2e-create-proposal-preview"]')
		await previewBtn.waitForClickable({ timeout: 60000 })
		await previewBtn.click()

		await $('//h1[contains(.,"Review")]').waitForDisplayed({ timeout: 60000 })
		const previewChange = await $('[data-testid="e2e-signer-set-change-table"]')
		await previewChange.waitForDisplayed({ timeout: 30000 })
		const previewChangeText = await previewChange.getText()
		expect(previewChangeText).toContain('+')
		expect(previewChangeText).toContain(newSigner)

		const signBtn = await $('button[data-testid="e2e-create-proposal-sign-submit"]')
		await signBtn.waitForClickable({ timeout: 60000 })
		await signBtn.click()

		const success = await $('[data-testid="e2e-proposal-signature-success"]')
		await success.waitForDisplayed({ timeout: 120000 })

		const continueBtn = await $('//button[normalize-space()="Continue →"]')
		await continueBtn.waitForClickable({ timeout: 30000 })
		await continueBtn.click()
		await browser.waitUntil(async () => !(await browser.getUrl()).includes('/create'), {
			timeout: 60000,
			timeoutMsg: 'Continue should return to the proposals dashboard',
		})

		const createdCard = await $(`//p[normalize-space()="${proposalTitle}"]/ancestor::div[contains(@class,"group")][1]`)
		await createdCard.waitForClickable({ timeout: 60000 })
		await createdCard.click()

		const detailChange = await $('[data-testid="e2e-signer-set-change-table"]')
		await detailChange.waitForDisplayed({ timeout: 60000 })
		expect(await detailChange.getText()).toBe(previewChangeText)
	})
})
