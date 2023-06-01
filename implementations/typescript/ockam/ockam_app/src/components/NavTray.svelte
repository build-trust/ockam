<script>
    import Logo from "./Logo.svelte";
</script>

<style>
.container {
    order: 1;
    flex-basis: auto;
    align-self: flex-start;
    flex: 1;

    width: 80px;
    max-width: 80px;
    height: 100%;
    border-radius: 0 15px 15px 0;
    padding: 20px;
    text-align: center;

    background-color: rgb(48, 48, 48);
    background-image: linear-gradient(rgb(48, 48, 48), rgb(56, 56, 56));
    box-shadow: 0px 0px 10px rgba(0, 0, 0, 0.15);
}
</style>

<div class="container">
    <Logo --width="40px"/>
</div>