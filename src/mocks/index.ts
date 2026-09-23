/**
 * Entry point of the mock backend. Importing it seeds the in-memory database exactly once.
 * Only module `services/` files may import from here.
 */
import { db, session } from './db';
import { seedDatabase } from './seed';

seedDatabase();

export { db, session };
export { ApiError, clone, delay, inDateRange, includesText, localDateKey, round2, sum, uid } from './utils';
