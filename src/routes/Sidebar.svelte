<script>
    import {AppRail, AppRailTile, AppRailAnchor} from "@skeletonlabs/skeleton";
    import {page} from "$app/stores";

</script>

<AppRail>
    <svelte:fragment slot="lead">
        <AppRailAnchor href="/">(icon)</AppRailAnchor>
    </svelte:fragment>
    <!-- --- -->
    <AppRailAnchor href="/" title="home" selected={$page.url.pathname === '/'}>
        <svelte:fragment slot="lead">(icon)</svelte:fragment>
        <span>查询</span>
    </AppRailAnchor>
    <AppRailAnchor href="/settings" title="settings" selected={$page.url.pathname === '/settings'}>
        <svelte:fragment slot="lead">(icon)</svelte:fragment>
        <span>设置</span>
    </AppRailAnchor>
    <!-- --- -->
    <svelte:fragment slot="trail">
        <AppRailAnchor href="https://google.com" target="_blank" title="About">(icon)</AppRailAnchor>
    </svelte:fragment>
</AppRail>
