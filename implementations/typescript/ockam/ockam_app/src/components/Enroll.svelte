<script>
  import Logo from "./Logo.svelte";
  import { invoke } from "@tauri-apps/api/tauri";

  export let enrolled = false;
  let waiting = false;
  let display = false
  async function show() {
    display = true
    await new Promise(r => setTimeout(r, 600))
  }
  async function hide() {
    display = false
    await new Promise(r => setTimeout(r, 600))
  }
  async function enroll(e) {
    waiting = true;
    e.target.disabled = true;
    await invoke('enroll')
    waiting = false
    await hide()
    enrolled = true
  }

  $: show()
</script>

<div class="container {display ? "in" : "out" }">
  <Logo />
  <h1>Welcome to Ockam</h1>
  <button on:click={enroll} class={waiting ? "waiting" : ""}>
    <span>Enroll</span>
  </button>
</div>

<style>
  @font-face {
    font-family: "Inter";
    src: url("Inter-Thin.ttf") format("truetype");
    font-weight: 100;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-ExtraLight.ttf") format("truetype");
    font-weight: 200;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-Light.ttf") format("truetype");
    font-weight: 300;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-Regular.ttf") format("truetype");
    font-weight: 400;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-Medium.ttf") format("truetype");
    font-weight: 500;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-SemiBold.ttf") format("truetype");
    font-weight: 600;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-Bold.ttf") format("truetype");
    font-weight: 700;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-ExtraBold.ttf") format("truetype");
    font-weight: 800;
  }
  @font-face {
    font-family: "Inter";
    src: url("Inter-Black.ttf") format("truetype");
    font-weight: 900;
  }
  button {
    position: relative;
    border: none;
    border-radius: 5px;
    padding: 10px 15px;
    margin: 10px;

    font-size: 14px;

    background: none;
    background-image: linear-gradient(
      rgba(255, 255, 255, 0.1),
      rgba(255, 255, 255, 0.05)
    );
    color: white;
  }
  button:hover {
    background-image: linear-gradient(
      rgba(255, 255, 255, 0.15),
      rgba(255, 255, 255, 0.1)
    );
    cursor: pointer;
  }
  button:active {
    background-image: linear-gradient(
      rgba(255, 255, 255, 0.1),
      rgba(255, 255, 255, 0.2)
    );
    box-shadow: inset 1px 1px 3px rgba(0, 0, 0, 0.2);
  }

  @keyframes spinner {
    to {
      transform: rotate(360deg);
    }
  }

  button.waiting {
    cursor: wait;
  }
  button.waiting span {
    visibility: hidden;
    opacity: 0;
  }
  button.waiting:before {
    content: "";
    visibility: visible;
    box-sizing: border-box;
    position: absolute;
    top: 50%;
    left: 50%;
    width: 20px;
    height: 20px;
    margin-top: -10px;
    margin-left: -10px;
    border-radius: 50%;
    border: 2px solid #ccc;
    border-top-color: rgba(255, 255, 255, 0);
    animation: spinner 0.6s linear infinite;
  }
  h1 {
    font-family: Inter;
    font-weight: 300;
    color: white;
  }

  .container {
    transition: opacity 0.55s ease-in-out;

    text-align: center;
    width: 100%;
    max-width: 400px;
    margin: 0 auto;
    padding: 40px 0;

    opacity: 0;
  }
  .container.out {
    opacity: 0;
  }
  .container.in {
    opacity: 1;
  }
</style>
