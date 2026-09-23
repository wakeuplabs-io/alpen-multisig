/** Shared Tailwind classes for create-proposal inputs (tabs / single quotes per project rules in TS strings). */
// TODO: refactor this

export const textInputClass =
	'mt-1.5 w-full rounded-lg border border-[#e5e7eb] px-3 py-2.5 text-body text-[#111827] placeholder:text-[#9ca3af] focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent'

const monoInputBase =
	'mt-1.5 w-full rounded-lg border border-[#e5e7eb] px-3 py-2.5 font-mono text-body text-[#111827] placeholder:text-[#9ca3af] focus:outline-none focus:ring-1'

export const monoInputClass = `${monoInputBase} focus:border-accent focus:ring-accent`

/** Same input, danger focus. Composed rather than appended: Tailwind resolves a `focus:border-*`
 * conflict by stylesheet order, so an override tacked onto `monoInputClass` may lose. */
export const monoInputDangerClass = `${monoInputBase} focus:border-danger focus:ring-danger`

export const numberInputClass =
	'mt-1.5 w-full rounded-lg border border-[#e5e7eb] px-3 py-2.5 text-body text-[#111827] focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent'

export const fieldErrorClass = 'mt-1 text-label text-danger-strong'
