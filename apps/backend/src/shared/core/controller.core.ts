import type { Request, Response, NextFunction } from "express";
import type { BaseService } from "./service.core";
import ApiResponse from "./response.core";
import { AsyncHandler } from "./asyncHandler";

/**
 * Thin pass-through CRUD controller, ported from the reference `BaseController`. Business rules never
 * live here — they live in the service. Extend and override only to add domain-specific endpoints.
 */
export class BaseController<T extends BaseService<any> = BaseService<any>> {
    Service: T;
    ModuleName: string;

    constructor(service: T, moduleName = "السجل") {
        this.Service = service;
        this.ModuleName = moduleName;
    }

    createDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const newDocument = await this.Service.createDocument(req.body);
        ApiResponse.success({ res, statusCode: 201, message: `تم إنشاء ${this.ModuleName} بنجاح`, data: newDocument });
    }, "BaseController.createDocument");

    readDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const result = await this.Service.readDocument({ query: req.query });
        ApiResponse.success({ res, data: result.data, page: result.page, limit: result.limit, total: result.total, totalPages: result.totalPages });
    }, "BaseController.readDocument");

    readDocumentById = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const document = await this.Service.requireDocumentById(req.params.id);
        ApiResponse.success({ res, data: document });
    }, "BaseController.readDocumentById");

    updateDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const updated = await this.Service.updateDocument({ id: req.params.id, data: req.body });
        ApiResponse.success({ res, message: `تم تحديث ${this.ModuleName} بنجاح`, data: updated });
    }, "BaseController.updateDocument");

    deleteDocument = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        await this.Service.deleteDocument({ id: req.params.id });
        ApiResponse.success({ res, message: `تم حذف ${this.ModuleName} بنجاح` });
    }, "BaseController.deleteDocument");
}

export default BaseController;
