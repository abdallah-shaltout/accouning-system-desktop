<script setup lang="ts">
import { computed } from "vue";
import { Badge } from "@/shared/components/ui/badge";
import { cn } from "@/shared/helpers/utils";

/**
 * A generic status badge (CLAUDE.md "Build from shared/components/app/*"). Never hardcodes business
 * statuses here — the caller passes its own status -> variant map (e.g. payments' pending/approved/
 * rejected, subscriptions' active/past_due/cancelled).
 */
export type StatusVariant = "neutral" | "success" | "warning" | "danger" | "info";

interface Props {
  status: string;
  label?: string;
  /** Maps a raw status value to a visual variant. Falls back to "neutral" when the status is missing. */
  variantMap?: Record<string, StatusVariant>;
}

const props = defineProps<Props>();

const variant = computed<StatusVariant>(() => props.variantMap?.[props.status] ?? "neutral");

const VARIANT_CLASSES: Record<StatusVariant, string> = {
  neutral: "bg-surface text-text-secondary border-border",
  success: "bg-success/10 text-success border-success/20",
  warning: "bg-warning/10 text-warning border-warning/20",
  danger: "bg-danger/10 text-danger border-danger/20",
  info: "bg-primary/10 text-primary border-primary/20",
};
</script>

<template>
  <Badge variant="outline" :class="cn('border font-normal', VARIANT_CLASSES[variant])">
    {{ label ?? status }}
  </Badge>
</template>
