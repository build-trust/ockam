<script>

</script>

<style>
.container {
    display: flex;
    flex-direction: column;
    flex-wrap: nowrap;
    justify-content: flex-start;
    align-items: flex-start;    
    align-content: flex-start;
    flex: 2;
    order: 2;

    height: 100%;
    padding: 10px;
    width: 100%;

}
</style>

<div class="container">
    <slot />
</div>