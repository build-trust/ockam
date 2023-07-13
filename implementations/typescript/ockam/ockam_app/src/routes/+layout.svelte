<script>
  import '../app.css'
</script>

<slot />

<style lang="postcss">
  :global(html) {
    background-color: theme(colors.primary.deep);
    color: theme(colors.natural.white)
  }
</style>
