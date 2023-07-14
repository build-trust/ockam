<script>
  import '../app.css'
  import Navbar from '../components/Navbar.svelte'
</script>

<Navbar />
<slot />

<style lang="postcss">
  :global(html) {
    background-color: theme(colors.primary.deep);
    color: theme(colors.natural.white)
  }
</style>
