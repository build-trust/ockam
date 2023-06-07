<script>
  import { AppRail, AppRailTile, AppRailAnchor } from "@skeletonlabs/skeleton";
  import Logo from "./Logo.svelte";
  import Cog from "svelte-material-icons/CogOutline.svelte";

  let currentTile = 0;
</script>

<AppRail>
  <svelte:fragment slot="lead">
    <AppRailAnchor href="/">
      <Logo --width="25px" />
    </AppRailAnchor>
  </svelte:fragment>
  <!-- ---
  <AppRailTile bind:group={currentTile} name="tile-1" value={0}>
    <svelte:fragment slot="lead">(icon)</svelte:fragment>
    <span>Tile 1</span>
  </AppRailTile>
  <AppRailTile bind:group={currentTile} name="tile-2" value={1}>
    <svelte:fragment slot="lead">(icon)</svelte:fragment>
    <span>Tile 2</span>
  </AppRailTile>
  <AppRailTile bind:group={currentTile} name="tile-3" value={2}>
    <svelte:fragment slot="lead">(icon)</svelte:fragment>
    <span>Tile 3</span>
  </AppRailTile> -->
  <!-- --- -->
  <svelte:fragment slot="trail">
    <AppRailAnchor href="/" target="_blank" title="Settings">
      <Cog size={25} class="icon" width="100%" color="rgba(255, 255, 255, 0.5)"/>
    </AppRailAnchor>
  </svelte:fragment>
</AppRail>

