<script>
  import { invoke } from '@tauri-apps/api/tauri'

  let visible = false;

  async function enroll() {
    invoke('enroll').then((message) => visible = true)
  }
</script>

<div>
  <button on:click="{enroll}">Enroll...</button>
  {#if visible}<p>Enrolled</p>{/if}
</div>
