<script>
  let enrolled = false;
  import Container from "../components/Container.svelte";
  import Main from "../components/Main.svelte";
  import Enroll from "../components/Enroll.svelte";
  import Portals from "../components/Portals.svelte";
  import Search from "../components/Search.svelte";
</script>

<Main>
  {#if !enrolled}
    <Enroll bind:enrolled />
  {:else}
    <Search />
    <Portals />
  {/if}
</Main>
