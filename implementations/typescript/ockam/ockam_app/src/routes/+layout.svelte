<script>
  import '../app.css'
  import '../assets/fonts/Inter-Light.ttf'
  import '../assets/fonts/Inter-Regular.ttf'
  import '../assets/fonts/Inter-Thin.ttf'
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
