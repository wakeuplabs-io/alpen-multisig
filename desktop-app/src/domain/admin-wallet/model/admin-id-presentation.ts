/**
 * Presentation rules for the Admin ID (PRD 06 §3.b.ii.2, §4.a).
 *
 * The Admin ID is a P2WPKH bitcoin address derived at `m/84'/0'/73'/0/0`. The rules
 * are shared with the connect flow, so they live in `@/lib/admin-id`; this module
 * re-exports them for the Admin Wallet domain.
 */
export { ADMIN_ID_LABEL, adminIdSafetyCaption, isDisplayableAdminId, truncateAdminId } from '@/lib/admin-id'
