<script>
  import PortalItem from "./PortalItem.svelte";

  const portals = [1, 2, 3, 4, 5, 6]
</script>

<dl class="list-dl">
    {#each portals as portal}
        <PortalItem />
    {/each}
</dl>

<style>
    dl {
        list-style: none;
        margin: 0;
        padding: 0;

        width: 100%;
    }
</style>