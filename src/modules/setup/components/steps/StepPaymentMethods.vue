<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 7: toggles pre-mapped to a system-role account (Phase 3's payment methods, reused). */
import { onMounted } from "vue";
import AppCard from "@/modules/core/components/ui/AppCard.vue";
import AppSwitch from "@/modules/core/components/ui/AppSwitch.vue";
import type { WizardState } from "../../types";

const props = defineProps<{ state: WizardState }>();

const DEFAULTS: {
    key: string;
    name: string;
    type: string;
    accountRole: string;
}[] = [
    { key: "cash", name: "نقداً", type: "cash", accountRole: "cash" },
    { key: "mada", name: "مدى", type: "card", accountRole: "cardClearing" },
    {
        key: "card",
        name: "فيزا / ماستركارد",
        type: "card",
        accountRole: "cardClearing",
    },
    {
        key: "bank_transfer",
        name: "تحويل بنكي",
        type: "bank_transfer",
        accountRole: "bank",
    },
    {
        key: "stc_pay",
        name: "STC Pay",
        type: "wallet",
        accountRole: "walletClearing",
    },
    { key: "credit", name: "آجل", type: "credit", accountRole: "receivable" },
];

onMounted(() => {
    if (!props.state.paymentMethods.length) {
        props.state.paymentMethods = DEFAULTS.map((d) => ({
            ...d,
            active: d.key === "cash" || d.key === "credit",
        }));
    }
});
</script>

<template>
    <AppCard>
        <div class="divide-y divide-border">
            <div
                v-for="m in state.paymentMethods"
                :key="m.key"
                class="flex items-center justify-between gap-3 py-3"
            >
                <div>
                    <AppSwitch v-model="m.active" :label="m.name" />
                </div>
                <span class="text-xs text-text-secondary"
                    >يُقيَّد على حساب دور «{{ m.accountRole }}» من شجرة
                    الحسابات</span
                >
            </div>
        </div>
    </AppCard>
</template>
