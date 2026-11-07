<script>
	import Game from '#lib/Game.svelte';
</script>

<svelte:head>
	<title>QBasic Gorillas</title>
</svelte:head>

<!-- Just the game, for framing in another page (games.markusmayer.dev). -->
<main>
	<Game />
</main>

<style>
	:global(html),
	:global(body) {
		margin: 0;
		height: 100%;
		background: #000;
	}
	main {
		height: 100%;
		display: grid;
		place-items: center;
	}
	main > :global(.screen) {
		width: min(100vw, calc(100vh * 4 / 3));
	}
</style>
