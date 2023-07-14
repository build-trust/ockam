<script>
  import { invoke } from '@tauri-apps/api/tauri'

  let visible = false;

  async function enroll() {
    invoke('enroll').then((message) => visible = true)
  }
</script>

<div class="m-2 max-w-full text-center">
  <button class="bg-secondary-light m-2 outline-natural-light p-2 rounded-lg text-natural-white" on:click="{enroll}">Enroll...</button>
  {#if visible}<p>Enrolled</p>{/if}
</div>
