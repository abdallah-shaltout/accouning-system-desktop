<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { Moon, Sun, Sparkles, Building2 } from "@lucide/vue";
import AppButton from "@/modules/core/components/ui/AppButton.vue";
import BrandLogo from "@/modules/core/components/ui/BrandLogo.vue";
import {
    resolvedTheme,
    toggleTheme,
} from "@/modules/core/controllers/useTheme";
import { errorMessage } from "@/modules/core/controllers/useToast";
import { reloadDemoData } from "@/modules/core/services/devToolsService";
import { APP_NAME_AR } from "@/modules/core/helpers/brand";

const router = useRouter();
const pending = ref<"demo" | "fresh" | null>(null);
const error = ref("");

async function startDemo() {
    error.value = "";
    pending.value = "demo";
    try {
        await reloadDemoData();
        router.replace({ name: "login" });
    } catch (err) {
        error.value = errorMessage(err);
    } finally {
        pending.value = null;
    }
}

// v2 phase 5 (docs/v2/05-onboarding.md): the 11-step onboarding wizard. It seeds the empty-company
// shell itself on mount (same `seedEmptyCompany()` this stub used to call directly) — see
// src/modules/setup/pages/SetupWizardPage.vue.
function startFresh() {
    router.push({ name: "setup-wizard" });
}
</script>

<template>
    <div
        class="flex min-h-screen items-center justify-center bg-background px-6 py-10"
    >
        <button
            type="button"
            class="fixed end-4 top-4 flex size-8 items-center justify-center rounded-md text-text-secondary hover:bg-surface-hover"
            aria-label="تبديل المظهر"
            @click="toggleTheme"
        >
            <Sun v-if="resolvedTheme === 'dark'" class="size-4" />
            <Moon v-else class="size-4" />
        </button>

        <div class="w-full max-w-2xl">
            <div class="mb-10 flex flex-col items-center text-center">
                <BrandLogo class="mb-5 h-10 w-auto" />
                <h1 class="text-xl font-semibold">مرحباً بك</h1>
                <p class="mt-1 text-body text-text-secondary">
                    {{ APP_NAME_AR }} — اختر كيف تبدأ
                </p>
            </div>

            <p
                v-if="error"
                class="mb-4 rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger"
                role="alert"
            >
                {{ error }}
            </p>

            <div class="grid gap-4 sm:grid-cols-2">
                <button
                    type="button"
                    class="group flex flex-col items-start gap-3 rounded-xl border border-border bg-surface p-6 text-start transition-colors hover:border-primary/50 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
                    :disabled="pending !== null"
                    @click="startFresh"
                >
                    <span
                        class="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary"
                    >
                        <Building2 class="size-5" />
                    </span>
                    <span class="text-sm font-semibold">ابدأ شركتك</span>
                    <span class="text-xs leading-relaxed text-text-secondary">
                        معالج إعداد من 11 خطوة: بيانات المنشأة، الضريبة، الفروع،
                        شجرة الحسابات، والأرصدة الافتتاحية.
                    </span>
                    <AppButton
                        variant="primary"
                        size="sm"
                        class="mt-1"
                        :disabled="pending !== null"
                        >ابدأ الآن</AppButton
                    >
                </button>

                <button
                    type="button"
                    class="group flex flex-col items-start gap-3 rounded-xl border border-border bg-surface p-6 text-start transition-colors hover:border-primary/50 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
                    :disabled="pending !== null"
                    @click="startDemo"
                >
                    <span
                        class="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary"
                    >
                        <Sparkles class="size-5" />
                    </span>
                    <span class="text-sm font-semibold"
                        >استكشف ببيانات تجريبية</span
                    >
                    <span class="text-xs leading-relaxed text-text-secondary">
                        حمّل متجراً تجريبياً كاملاً (منتجات، عملاء، فواتير
                        وحسابات) لتجربة النظام فوراً.
                    </span>
                    <AppButton
                        variant="secondary"
                        size="sm"
                        class="mt-1"
                        :loading="pending === 'demo'"
                        :disabled="pending !== null"
                        >تحميل البيانات</AppButton
                    >
                </button>
            </div>
        </div>
    </div>
</template>
