<script setup lang="ts">
/**
 * Mount this ONCE near the app root (App.vue). Every destructive action then calls
 * `const { confirm } = useConfirmDialog()` and awaits a boolean (CLAUDE.md "Destructive actions go
 * through ConfirmDialog"). The composable and this component share one small reactive state module
 * so any page can request a confirmation without prop-drilling a dialog instance.
 */
import AppButton from "./AppButton.vue";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/shared/components/ui/dialog";
import { confirmState, resolveConfirm } from "./useConfirmDialog";
</script>

<template>
  <Dialog :open="confirmState.open" @update:open="(open) => !open && resolveConfirm(false)">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>{{ confirmState.options.title }}</DialogTitle>
        <DialogDescription>{{ confirmState.options.message }}</DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <AppButton variant="outline" @click="resolveConfirm(false)">
          {{ confirmState.options.cancelLabel }}
        </AppButton>
        <AppButton
          :variant="confirmState.options.destructive ? 'destructive' : 'default'"
          @click="resolveConfirm(true)"
        >
          {{ confirmState.options.confirmLabel }}
        </AppButton>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
