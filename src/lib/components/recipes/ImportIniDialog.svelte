<script lang="ts">
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import Check from '@lucide/svelte/icons/check';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Loader2 from '@lucide/svelte/icons/loader-2';
  import Search from '@lucide/svelte/icons/search';
  import Sliders from '@lucide/svelte/icons/sliders';
  import { toast } from 'svelte-sonner';

  import { importIniPresets, previewIniPresets } from '$lib/api/recipes';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Input } from '$lib/components/ui/input';
  import { appStore } from '$lib/stores/app.svelte';
  import { recipesStore } from '$lib/stores/recipes.svelte';
  import type { ParsedIniFile } from '$lib/types';
  import { notifyError } from '$lib/utils/notify';

  let loading = $state(false);
  let importing = $state(false);
  let parsed = $state<ParsedIniFile | null>(null);
  let remaps = $state<Record<string, string>>({});
  let selected = $state<Record<string, boolean>>({});
  let searchQuery = $state('');

  const currentPath = $derived(appStore.importIniInitialPath);
  const rootVarKeys = $derived(parsed ? Object.keys(parsed.root_variables) : []);

  $effect(() => {
    if (appStore.importIniOpen && currentPath) {
      loadPreview(currentPath);
    } else if (!appStore.importIniOpen) {
      parsed = null;
      remaps = {};
      selected = {};
      searchQuery = '';
    }
  });

  async function loadPreview(path: string) {
    loading = true;
    try {
      const data = await previewIniPresets(path);
      parsed = data;
      const initialRemaps: Record<string, string> = {};
      for (const [k, v] of Object.entries(data.root_variables)) {
        initialRemaps[k] = v;
      }
      remaps = initialRemaps;

      const initialSelected: Record<string, boolean> = {};
      for (const p of data.presets) {
        initialSelected[p.name] = true;
      }
      selected = initialSelected;
    } catch (e) {
      notifyError(e);
      appStore.importIniOpen = false;
    } finally {
      loading = false;
    }
  }

  async function browseFolderForVar(varName: string) {
    try {
      const dir = await openDialog({
        directory: true,
        defaultPath: remaps[varName] || undefined,
      });
      if (typeof dir === 'string' && dir) {
        remaps[varName] = dir;
      }
    } catch (e) {
      notifyError(e);
    }
  }

  const filteredPresets = $derived(
    parsed
      ? parsed.presets.filter(
          (p) =>
            p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            p.model_path.toLowerCase().includes(searchQuery.toLowerCase()) ||
            p.description.toLowerCase().includes(searchQuery.toLowerCase()),
        )
      : [],
  );

  const selectedCount = $derived(Object.values(selected).filter(Boolean).length);
  const totalCount = $derived(parsed ? parsed.presets.length : 0);
  const allFilteredSelected = $derived(
    filteredPresets.length > 0 && filteredPresets.every((p) => selected[p.name]),
  );

  function toggleAll() {
    const nextState = !allFilteredSelected;
    for (const p of filteredPresets) {
      selected[p.name] = nextState;
    }
  }

  function toggleOne(name: string) {
    selected[name] = !selected[name];
  }

  async function handleImport() {
    if (!currentPath || selectedCount === 0) return;
    importing = true;
    try {
      const selectedNames = Object.keys(selected).filter((k) => selected[k]);
      const count = await importIniPresets(currentPath, remaps, selectedNames);
      await recipesStore.refresh();
      toast.success(
        count === 1
          ? 'Imported 1 recipe from INI preset'
          : `Imported ${count} recipes from INI preset`,
      );
      appStore.importIniOpen = false;
    } catch (e) {
      notifyError(e);
    } finally {
      importing = false;
    }
  }
</script>

<Dialog.Root bind:open={appStore.importIniOpen}>
  <Dialog.Content class="flex max-h-[85vh] flex-col sm:max-w-2xl">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <Sliders class="size-5 text-primary" />
        Import INI Presets
      </Dialog.Title>
      <Dialog.Description class="truncate">
        {#if currentPath}
          File: <span class="font-mono text-xs font-semibold text-foreground">{currentPath}</span>
        {:else}
          Select and import recipes from a llama-server --models-preset file.
        {/if}
      </Dialog.Description>
    </Dialog.Header>

    {#if loading}
      <div class="flex flex-1 items-center justify-center py-12 text-muted-foreground">
        <Loader2 class="mr-2 size-5 animate-spin" />
        Parsing presets configuration...
      </div>
    {:else if parsed}
      <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
        <!-- Root Variables Remapping -->
        {#if rootVarKeys.length > 0}
          <div class="rounded-md border bg-muted/30 p-3 text-xs">
            <div class="mb-2 font-semibold text-foreground">
              Detected Path Variables (Adjust to match local directory):
            </div>
            <div class="flex flex-col gap-2">
              {#each rootVarKeys as varName (varName)}
                <div class="flex items-center gap-2">
                  <span class="w-32 shrink-0 font-mono font-medium text-foreground">
                    &#123;{varName}&#125;
                  </span>
                  <Input
                    bind:value={remaps[varName]}
                    class="h-7 text-xs font-mono"
                    placeholder="Path on local system..."
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    class="h-7 shrink-0 px-2 text-xs"
                    onclick={() => browseFolderForVar(varName)}
                  >
                    <FolderOpen class="mr-1 size-3.5" />
                    Browse
                  </Button>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Presets Selection Bar -->
        <div class="flex items-center justify-between gap-2">
          <div class="relative flex-1">
            <Search
              class="absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              bind:value={searchQuery}
              placeholder="Filter models by name or path..."
              class="h-8 pl-7 text-xs"
            />
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            class="h-8 text-xs text-muted-foreground"
            onclick={toggleAll}
          >
            {allFilteredSelected ? 'Deselect all' : 'Select all'}
          </Button>
        </div>

        <!-- Scrollable List of Presets -->
        <div class="flex-1 overflow-y-auto rounded-md border divide-y">
          {#each filteredPresets as preset (preset.name)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="flex cursor-pointer items-start gap-3 p-2.5 transition-colors hover:bg-muted/50 {selected[
                preset.name
              ]
                ? 'bg-primary/5'
                : ''}"
              onclick={() => toggleOne(preset.name)}
            >
              <div
                class="mt-0.5 flex size-4 shrink-0 items-center justify-center rounded border {selected[
                  preset.name
                ]
                  ? 'border-primary bg-primary text-primary-foreground'
                  : 'border-input bg-background'}"
              >
                {#if selected[preset.name]}
                  <Check class="size-3 stroke-[3]" />
                {/if}
              </div>

              <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                <div class="flex items-center gap-2">
                  <span class="truncate text-xs font-semibold text-foreground">{preset.name}</span>
                  {#if preset.tags}
                    <Badge variant="outline" class="h-4 px-1 text-[10px]">
                      {preset.tags}
                    </Badge>
                  {/if}
                </div>
                <div class="truncate font-mono text-[11px] text-muted-foreground">
                  {preset.model_path}
                </div>
                {#if preset.description}
                  <div class="line-clamp-1 text-[11px] text-muted-foreground/80">
                    {preset.description}
                  </div>
                {/if}
              </div>
            </div>
          {:else}
            <div class="py-8 text-center text-xs text-muted-foreground">
              No presets match your search.
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <Dialog.Footer class="flex items-center justify-between sm:justify-between pt-2">
      <div class="text-xs text-muted-foreground">
        {#if parsed}
          <span class="font-medium text-foreground">{selectedCount}</span> of {totalCount} selected
        {/if}
      </div>
      <div class="flex items-center gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={importing}
          onclick={() => (appStore.importIniOpen = false)}
        >
          Cancel
        </Button>
        <Button
          type="button"
          size="sm"
          disabled={loading || importing || selectedCount === 0}
          onclick={handleImport}
        >
          {#if importing}
            <Loader2 class="mr-1.5 size-3.5 animate-spin" />
            Importing...
          {:else}
            Import {selectedCount} {selectedCount === 1 ? 'Recipe' : 'Recipes'}
          {/if}
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
