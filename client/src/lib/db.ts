import { openDB, type DBSchema, type IDBPDatabase } from 'idb';
import type { SemanticData } from './types';

export type { SemanticData } from './types';

export interface DocumentMeta {
    id: string;
    fileName: string;
    createdAt: number;
    size: number;
}

interface MDFSchema extends DBSchema {
    documents: {
        key: string;
        value: DocumentMeta;
        indexes: { 'by-date': number };
    };
    content: {
        key: string;
        value: { id: string; data: SemanticData; rawText?: string };
    };
}

let dbPromise: Promise<IDBPDatabase<MDFSchema>> | null = null;

export async function getDB() {
    if (!dbPromise) {
        dbPromise = openDB<MDFSchema>('mdf_storage', 1, {
            upgrade(db) {
                const docStore = db.createObjectStore('documents', { keyPath: 'id' });
                docStore.createIndex('by-date', 'createdAt');
                
                db.createObjectStore('content', { keyPath: 'id' });
            },
        });
    }
    return dbPromise;
}

export async function saveDocument(fileName: string, size: number, data: SemanticData, rawText?: string): Promise<string> {
    const db = await getDB();
    const id = crypto.randomUUID();
    const now = Date.now();
    
    const meta: DocumentMeta = {
        id,
        fileName,
        size,
        createdAt: now
    };
    
    const tx = db.transaction(['documents', 'content'], 'readwrite');
    await Promise.all([
        tx.objectStore('documents').put(meta),
        tx.objectStore('content').put({ id, data, rawText }),
        tx.done
    ]);
    
    return id;
}

export async function getAllDocuments(): Promise<DocumentMeta[]> {
    const db = await getDB();
    const docs = await db.getAllFromIndex('documents', 'by-date');
    return docs.reverse();
}

export async function getDocumentContent(id: string): Promise<{ data: SemanticData; rawText?: string } | null> {
    const db = await getDB();
    const entry = await db.get('content', id);
    return entry || null;
}

export async function deleteDocument(id: string): Promise<void> {
    const db = await getDB();
    const tx = db.transaction(['documents', 'content'], 'readwrite');
    await Promise.all([
        tx.objectStore('documents').delete(id),
        tx.objectStore('content').delete(id),
        tx.done
    ]);
}
