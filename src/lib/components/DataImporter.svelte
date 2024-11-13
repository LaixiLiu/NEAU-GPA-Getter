<script>
    import { invoke } from "@tauri-apps/api/core";
    import { ProgressBar } from "@skeletonlabs/skeleton";
    import fileOpen from "$lib/images/file-open.png";
    import nextStep from "$lib/images/next-step.png";
    import { getToastStore } from "@skeletonlabs/skeleton";

    const toastStore = getToastStore();
    let isInitialized = false;

    let promise_set = undefined;

    function initializeData() {
        promise_set = invoke("initialize_searcher");
    }

    function setInitialized() {
        isInitialized = true;
        return true;
    }
</script>

<div class="flex flex-col items-center">
    <button
        type="button"
        class="btn variant-filled flex items-center mb-2"
        disabled={isInitialized}
        on:click={initializeData}
    >
        <img src={fileOpen} alt="file-open" class="h-full size-6" />
        <span>导入数据以开始</span>
    </button>
    {#await promise_set}
        <ProgressBar />
    {:then message}
        {#if message !== undefined && toastStore.trigger( { message }, ) && setInitialized()}
            <button
                type="button"
                class="btn variant-filled flex items-center mb-2 background-green"
                on:click={() => {
                    // reload the page
                    location.reload();
                }}
            >
                <img src={nextStep} alt="next-step" class="h-full size-6" />
                <span>开始查询</span>
            </button>
        {/if}
    {:catch error}
        {toastStore.trigger({ message: error })}
    {/await}
</div>
