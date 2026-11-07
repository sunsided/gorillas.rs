<script>
	import { onMount } from 'svelte';

	/** @type {HTMLCanvasElement | undefined} */
	let canvas = $state();
	let status = $state('loading');
	let error = $state('');

	onMount(async () => {
		try {
			const wasm = await import('#lib/wasm/gorillas.js');
			await wasm.default();
			status = 'running';
			canvas?.focus();
			wasm.start('gorillas');
		} catch (e) {
			// winit unwinds out of its event loop setup with this exception on the web; the game
			// keeps running.
			const message = e instanceof Error ? e.message : String(e);
			if (message.includes('Using exceptions for control flow')) return;
			status = 'failed';
			error = message;
		}
	});
</script>

<svelte:head>
	<title>QBasic Gorillas</title>
</svelte:head>

<main>
	<header>
		<h1>QBasic Gorillas</h1>
		<p class="sub">GORILLAS.BAS (IBM, 1991), ported statement by statement to Rust</p>
	</header>

	<div class="screen">
		<canvas
			id="gorillas"
			bind:this={canvas}
			tabindex="0"
			aria-label="QBasic Gorillas game screen"
			onclick={() => canvas?.focus()}
		></canvas>
		{#if status !== 'running'}
			<div class="overlay" role="status">
				{#if status === 'loading'}
					Loading…
				{:else}
					Could not start: {error}
				{/if}
			</div>
		{/if}
	</div>

	<section class="help">
		<p>
			Click the screen to give it the keyboard (and the PC speaker its audio). Type names,
			points and gravity, then <kbd>V</kbd> for the intro or <kbd>P</kbd> to play. Each turn,
			enter the angle and velocity of your throw.
		</p>
		<p>
			<kbd>Alt</kbd>+<kbd>C</kbd> cycles CRT filters, <kbd>Alt</kbd>+<kbd>Enter</kbd> goes
			fullscreen.
		</p>
	</section>

	<footer>
		<p>
			The original runs in QBasic; this one runs the same statements on an emulated VGA card and
			PC speaker (<a href="https://github.com/sunsided/game-toolkit-dosvideo"
				>game-toolkit-dosvideo</a
			>), with QBasic's line, circle, random number and <code>PLAY</code> quirks, checked pixel by
			pixel against QBasic 1.1 in DOSBox.
		</p>
		<p>
			<a href="https://github.com/sunsided/gorillas.rs">Source</a> · GORILLAS.BAS © IBM Corporation
			1991
		</p>
	</footer>
</main>

<style>
	:global(html) {
		color-scheme: dark;
	}
	:global(body) {
		margin: 0;
		background: #0b0b12;
		color: #c8c8d0;
		font-family: ui-monospace, 'Cascadia Mono', 'DejaVu Sans Mono', monospace;
		line-height: 1.5;
	}
	main {
		max-width: 1024px;
		margin: 0 auto;
		padding: 24px 16px 48px;
	}
	header {
		text-align: center;
		margin-bottom: 16px;
	}
	h1 {
		margin: 0;
		font-size: 1.6rem;
		letter-spacing: 0.4em;
		text-transform: uppercase;
		color: #fff;
	}
	.sub {
		margin: 4px 0 0;
		color: #8a8a99;
		font-size: 0.85rem;
	}
	.screen {
		position: relative;
		aspect-ratio: 4 / 3;
		background: #000;
		box-shadow: 0 0 0 1px #2a2a3a;
	}
	canvas {
		display: block;
		width: 100%;
		height: 100%;
		outline: none;
		touch-action: none;
	}
	canvas:focus-visible {
		box-shadow: 0 0 0 2px #5555ff;
	}
	.overlay {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		color: #aaa;
	}
	.help,
	footer {
		font-size: 0.85rem;
	}
	.help {
		margin-top: 16px;
	}
	footer {
		margin-top: 24px;
		color: #8a8a99;
	}
	kbd {
		padding: 0 4px;
		border: 1px solid #44445a;
		border-radius: 3px;
		background: #16161f;
		color: #fff;
	}
	a {
		color: #8888ff;
	}
</style>
