import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { userService } from "@@user/user.service";
import { hashPassword } from "@@shared/utils/password";

function toPublicUser(row: {
    id: string;
    orgId: string;
    name: string;
    phone: string;
    email: string | null;
    role: "owner" | "member";
    active: boolean;
    phoneVerifiedAt: Date | null;
    createdAt: Date;
}) {
    return {
        id: row.id,
        orgId: row.orgId,
        name: row.name,
        phone: row.phone,
        email: row.email,
        role: row.role,
        active: row.active,
        phoneVerifiedAt: row.phoneVerifiedAt,
        createdAt: row.createdAt,
    };
}

/**
 * Portal users surface: owner-only management of other portal logins in their own org. The org's
 * maxUsers entitlement does NOT apply here (these are portal logins, not desktop users) — a
 * deliberate B1 decision, not an oversight.
 */
class PortalUserController {
    list = AsyncHandler(async (req: Request, res: Response) => {
        const result = await userService.readDocument({
            query: req.query,
            reqFilter: { orgId: req.orgId },
        });
        ApiResponse.success({
            res,
            data: result.data.map(toPublicUser),
            page: result.page,
            limit: result.limit,
            total: result.total,
            totalPages: result.totalPages,
        });
    }, "PortalUserController.list");

    create = AsyncHandler(async (req: Request, res: Response) => {
        const { name, phone, password } = req.body as { name: string; phone: string; password: string };

        const alreadyRegistered = await userService.isPhoneRegistered(phone);
        if (alreadyRegistered) {
            throw new ApiError({ statusCode: 409, message: "رقم الهاتف مستخدم بالفعل", code: "conflict" });
        }

        const passwordHash = await hashPassword(password);

        // The owner vouches for this phone, so it's trusted immediately — no OTP round-trip.
        const created = await userService.createUser({
            orgId: req.orgId!,
            name,
            phone,
            passwordHash,
            role: "member",
            phoneVerifiedAt: new Date(),
        });

        ApiResponse.success({
            res,
            statusCode: 201,
            message: "تم إضافة المستخدم بنجاح",
            data: toPublicUser(created),
        });
    }, "PortalUserController.create");

    remove = AsyncHandler(async (req: Request, res: Response) => {
        const target = await userService.readDocumentById(req.params.id);
        if (!target || target.orgId !== req.orgId) {
            throw new ApiError({ statusCode: 404, message: "المستخدم غير موجود", code: "not_found" });
        }

        if (target.role === "owner") {
            const ownerCount = await userService.countOwners(req.orgId!);
            if (ownerCount <= 1) {
                throw new ApiError({
                    statusCode: 409,
                    message: "لا يمكن حذف آخر مالك في المنشأة",
                    code: "conflict",
                });
            }
        }

        await userService.deleteDocument({ id: target.id });
        ApiResponse.success({ res, message: "تم حذف المستخدم بنجاح" });
    }, "PortalUserController.remove");
}

export const portalUserController = new PortalUserController();
export default portalUserController;
