import { FormEvent, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export default function App() {
	const [name, setName] = useState('')
	const [message, setMessage] = useState('Hello from React')

	const handleGreet = async (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault()
		const greeting = await invoke<string>('greet', { name })
		setMessage(greeting)
	}

	return (
		<main
			style={{
				minHeight: '100vh',
				display: 'grid',
				placeItems: 'center',
				fontFamily: 'Inter, system-ui, sans-serif',
				padding: '2rem',
			}}
		>
			<section style={{ width: '100%', maxWidth: 420 }}>
				<h1 style={{ marginBottom: '0.5rem' }}>React + Tauri + Rust</h1>
				<p style={{ marginTop: 0, color: '#666' }}>Hello world example</p>

				<form onSubmit={handleGreet} style={{ display: 'grid', gap: '0.75rem', marginTop: '1rem' }}>
					<input
						value={name}
						onChange={(event) => setName(event.target.value)}
						placeholder="Enter your name"
						style={{ padding: '0.6rem 0.75rem', borderRadius: 8, border: '1px solid #ccc' }}
					/>
					<button type="submit" style={{ padding: '0.6rem 0.75rem', borderRadius: 8, border: '1px solid #222' }}>
						Say hello from Rust
					</button>
				</form>

				<p style={{ marginTop: '1rem' }}>{message}</p>
			</section>
		</main>
	)
}
