import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { paymentService } from "@@payment/payment.service";

class PortalPaymentController {
    submit = AsyncHandler(async (req: Request, res: Response) => {
        const { invoiceId, method, reference } = req.body as { invoiceId: string; method: any; reference?: string };
        const file = (req as any).file as { buffer: Buffer; originalname: string; mimetype: string } | undefined;

        const created = await paymentService.submitManualPayment({
            orgId: req.orgId!,
            invoiceId,
            method,
            reference,
            receiptFile: file ? { buffer: file.buffer, originalName: file.originalname, mimeType: file.mimetype } : null,
        });

        ApiResponse.success({ res, statusCode: 201, message: "تم إرسال إثبات الدفع — بانتظار المراجعة", data: created });
    }, "PortalPaymentController.submit");

    list = AsyncHandler(async (req: Request, res: Response) => {
        const payments = await paymentService.listForOrg(req.orgId!);
        ApiResponse.success({ res, data: payments });
    }, "PortalPaymentController.list");

    listInvoices = AsyncHandler(async (req: Request, res: Response) => {
        const invoices = await paymentService.listInvoicesForOrg(req.orgId!);
        ApiResponse.success({ res, data: invoices });
    }, "PortalPaymentController.listInvoices");

    paymentInstructions = AsyncHandler(async (_req: Request, res: Response) => {
        let instructions: unknown = {};
        try {
            instructions = JSON.parse(process.env.PAYMENT_INSTRUCTIONS_JSON || "{}");
        } catch {
            instructions = {};
        }
        ApiResponse.success({ res, data: instructions });
    }, "PortalPaymentController.paymentInstructions");
}

export const portalPaymentController = new PortalPaymentController();
export default portalPaymentController;
