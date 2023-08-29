<script>
  import CreateServiceInvitation from "./CreateServiceInvitation.svelte";

  /** @type {import('./$types').PageData} */
  export let data;
</script>

<CreateServiceInvitation {...data} />
