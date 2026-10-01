import { v4 as uuidv4, validate } from "uuid";

export const generateUUID = (): string => uuidv4();
export const validateUUID = (uuid: string): boolean => validate(uuid);
