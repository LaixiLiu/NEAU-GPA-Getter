<script>
    import {invoke} from "@tauri-apps/api/core";
    import {ProgressRadial} from "@skeletonlabs/skeleton";

    let promise = null;

    function handleClick() {
        promise = invoke("initialize_searcher");
    }

</script>

<div class="file-selector">
    <button type="button" class="btn btn-md variant-filled" on:click={handleClick}>
        <span>(icon)</span>
        <span>请选择文件目录</span>
    </button>

    {#await promise}
        <ProgressRadial/>
    {:then value}
        <div class="bg-green-400">
            <p>Operation successful: {value}</p>
        </div>
    {:catch error}
        <div class="bg-red-400">
            <p>Error: {error.message}</p>
        </div>
    {/await}


</div>

<style>
</style>
