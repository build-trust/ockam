<script>
  import { AppShell } from "@skeletonlabs/skeleton";
  import NavTray from "../components/NavTray.svelte";

  import "@skeletonlabs/skeleton/themes/theme-skeleton.css";
  import "@skeletonlabs/skeleton/styles/skeleton.css";
  import "../app.postcss";
</script>

<AppShell>
	<svelte:fragment slot="sidebarLeft">
    <NavTray />
  </svelte:fragment>
	<slot />
</AppShell>