<script>
  import Magnify from "svelte-material-icons/Magnify.svelte";
  import Cmd from "svelte-material-icons/AppleKeyboardCommand.svelte"
  import Plus from "svelte-material-icons/Plus.svelte"
  import K from "svelte-material-icons/AlphaKBoxOutline.svelte"
</script>

<div class="input-group grid-cols-[auto_1fr_auto]">
  <div class="input-group-shim"><Magnify /></div>
  <input type="search" placeholder="Search..." />
  <div><kbd class="kbd keys">⌘ + K</kbd></div>
</div>

<style>
	.keys {
		opacity: 0.5;
	}
</style>
