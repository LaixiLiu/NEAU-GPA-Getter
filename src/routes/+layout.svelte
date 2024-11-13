<script>
    import "../app.css";
    import { initializeStores, Toast } from "@skeletonlabs/skeleton";

    initializeStores();
</script>

<div class="grid h-screen">
    <div class="grid">
        <main class="col-auto p-4 mx-auto box-border">
            <slot></slot>
        </main>
        <Toast />
    </div>
</div>
