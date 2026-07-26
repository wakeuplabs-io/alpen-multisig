/**
 * Presentation rules for the Admin ID (PRD §4.1, as corrected by issue #408).
 *
 * The Admin ID is the signer's **compressed public key**, not a Bitcoin address.
 * The rules are shared with the connect flow, so they live in `@/lib/admin-id`;
 * this module re-exports them for the Admin Wallet domain.
 */
export { ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION, isDisplayableAdminId, truncateAdminId } from '@/lib/admin-id'
