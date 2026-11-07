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

<style>
	.screen {
		position: relative;
		aspect-ratio: 4 / 3;
		overflow: hidden;
		background: #000;
	}
	canvas {
		display: block;
		/* The toolkit sizes the canvas inline (1024x768); the frame decides here. */
		width: 100% !important;
		height: 100% !important;
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
		font-family: ui-monospace, monospace;
	}
</style>
