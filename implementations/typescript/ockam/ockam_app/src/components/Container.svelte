<script>

</script>

<style>
.container {
    display: flex;
    flex-direction: row;
    flex-wrap: nowrap;
    justify-content: flex-start;
    align-items: flex-start;    
    align-content: flex-start;

    width: 100%;
    height: 100%;

    background-color: rgb(33, 33, 33);
    background-image: linear-gradient(rgb(33, 33, 33), rgb(36, 39, 52));
}
</style>

<div class="container">
    <slot />
</div>