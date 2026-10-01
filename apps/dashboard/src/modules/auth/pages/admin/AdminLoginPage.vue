<script setup lang="ts">
import { ref } from "vue";
import { useForm } from "vee-validate";
import { toTypedSchema } from "@vee-validate/zod";
import { useRoute, useRouter } from "vue-router";
import { toast } from "vue-sonner";
import { AppButton, FormField } from "@/shared/components/app";
import { ApiError, errorMessageFor } from "@/shared/api/errors";
import { adminLoginSchema, adminTotpSchema } from "../../schemas/adminAuth.schema";
import { authService } from "../../services/authService";

/**
 * Admin login: email + password -> TOTP step (docs/04-screens.md `admin-login`). Proves the full
 * scaffold end-to-end (D1) — this module is properly built out in D2; only the login/TOTP flow lives
 * here for now.
 */
const router = useRouter();
const route = useRoute();

const step = ref<"credentials" | "totp">("credentials");
const challengeId = ref("");
const submitting = ref(false);

const credentialsForm = useForm({ validationSchema: toTypedSchema(adminLoginSchema) });
const totpForm = useForm({ validationSchema: toTypedSchema(adminTotpSchema) });

const submitCredentials = credentialsForm.handleSubmit(async (values) => {
  submitting.value = true;
  try {
    const result = await authService.adminLogin(values);
    challengeId.value = result.challengeId;
    totpForm.setFieldValue("challengeId", result.challengeId);
    step.value = "totp";
  } catch (err) {
    toast.error(errorMessageFor(err instanceof ApiError ? err : null));
  } finally {
    submitting.value = false;
  }
});

const submitTotp = totpForm.handleSubmit(async (values) => {
  submitting.value = true;
  try {
    await authService.adminTotp({ ...values, challengeId: challengeId.value });
    const redirect = typeof route.query.redirect === "string" ? route.query.redirect : undefined;
    if (redirect) {
      await router.push(redirect); /* route-ok: login redirect query is a URL round-trip, not a hard-coded target */
    } else {
      await router.push({ name: "admin-home" });
    }
  } catch (err) {
    toast.error(errorMessageFor(err instanceof ApiError ? err : null));
  } finally {
    submitting.value = false;
  }
});
</script>

<template>
  <form v-if="step === 'credentials'" class="flex flex-col gap-4" @submit="submitCredentials">
    <h2 class="text-lead font-semibold text-text-primary">تسجيل الدخول</h2>
    <FormField name="email" label="البريد الإلكتروني" type="email" autocomplete="username" dir="ltr" />
    <FormField name="password" label="كلمة المرور" type="password" autocomplete="current-password" />
    <AppButton type="submit" class="w-full" :loading="submitting">دخول</AppButton>
  </form>

  <form v-else class="flex flex-col gap-4" @submit="submitTotp">
    <h2 class="text-lead font-semibold text-text-primary">رمز التحقق</h2>
    <p class="text-body text-text-secondary">أدخل رمز التحقق المكوّن من 6 أرقام من تطبيق المصادقة</p>
    <FormField name="code" label="رمز التحقق" dir="ltr" autocomplete="one-time-code" />
    <AppButton type="submit" class="w-full" :loading="submitting">دخول</AppButton>
  </form>
</template>
