<script>
  import Logo from './Logo.svelte';
  import { invoke } from '@tauri-apps/api/tauri'

  let visible = false;

  async function enroll() {
    invoke('enroll').then((message) => visible = true)
  }
</script>

<style>

@font-face {
    font-family: 'Inter';
    src: url('Inter-Thin.ttf')  format('truetype');
    font-weight: 100    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-ExtraLight.ttf')  format('truetype');
    font-weight: 200    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-Light.ttf')  format('truetype');
    font-weight: 300    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-Regular.ttf')  format('truetype');
    font-weight: 400    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-Medium.ttf')  format('truetype');
    font-weight: 500    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-SemiBold.ttf')  format('truetype');
    font-weight: 600    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-Bold.ttf')  format('truetype');
    font-weight: 700    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-ExtraBold.ttf')  format('truetype');
    font-weight: 800    
}
@font-face {
    font-family: 'Inter';
    src: url('Inter-Black.ttf')  format('truetype');
    font-weight: 900    
}
  button {
    border: none;
    border-radius: 5px;
    padding: 10px 15px;
    margin: 10px;

    font-size: 14px;

    background: none;
    background-image: linear-gradient(rgba(255, 255, 255, 0.1), rgba(255, 255, 255, 0.05));
    color: white;
  }
  button:hover {
    background-image: linear-gradient(rgba(255, 255, 255, 0.15), rgba(255, 255, 255, 0.1)); 
    cursor: pointer;
  }
  button:active {
    background-image: linear-gradient(rgba(255, 255, 255, 0.1), rgba(255, 255, 255, 0.2));
    box-shadow: inset 1px 1px 3px rgba(0, 0, 0, 0.2);
  }

  h1 {
    font-family: Inter;
    font-weight: 300;
    color: white
  }

  div {
    text-align: center;
    width: 100%;
    max-width: 400px;
    margin: 0 auto;
    padding: 40px 0
  }
</style>

<div>
  <Logo />
  <h1>Welcome to Ockam</h1>
  <button on:click="{enroll}">Enroll...</button>
  {#if visible}<p>Enrolled</p>{/if}
</div>
