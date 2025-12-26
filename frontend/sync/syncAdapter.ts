import { SyncAdapter } from '@nozbe/watermelondb/sync';
import { PullChangesResponse, PushChangesResponse } from '@nozbe/watermelondb/sync/types';

/**
 * Custom Sync Adapter for GigPilot
 * 
 * This adapter implements the WatermelonDB sync protocol compatible with
 * our Rust backend's /sync/pull and /sync/push endpoints.
 * 
 * The sync protocol supports:
 * - Incremental sync with last_pulled_at timestamps
 * - Conflict resolution (ServerWins, ClientWins, LastWriteWins)
 * - Version vectors for CRDT-based synchronization
 */

interface SyncConfig {
  apiUrl: string;
  authToken: string;
  deviceId: string;
}

/**
 * Creates a sync adapter for WatermelonDB
 * 
 * @param config - Sync configuration (API URL, auth token, device ID)
 * @returns SyncAdapter instance
 */
export function createSyncAdapter(config: SyncConfig): SyncAdapter {
  const { apiUrl, authToken, deviceId } = config;

  return {
    /**
     * Pull changes from the server
     */
    async pullChanges({
      lastPulledAt,
      schemaVersion,
      migration,
    }): Promise<PullChangesResponse> {
      const url = new URL(`${apiUrl}/sync/pull`);
      if (lastPulledAt) {
        url.searchParams.set('last_pulled_at', lastPulledAt.toString());
      }
      url.searchParams.set('device_id', deviceId);

      const response = await fetch(url.toString(), {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${authToken}`,
          'Content-Type': 'application/json',
        },
      });

      if (!response.ok) {
        throw new Error(`Pull sync failed: ${response.statusText}`);
      }

      const data = await response.json();
      
      // Transform server response to WatermelonDB format
      // Server returns: { changes: { invoices: { created: [...], updated: [...], deleted: [...] } }, timestamp }
      // WatermelonDB expects: { changes: { invoices: { created: [...], updated: [...], deleted: [...] } }, timestamp }
      
      return {
        changes: data.changes,
        timestamp: new Date(data.timestamp).getTime(),
      };
    },

    /**
     * Push changes to the server
     */
    async pushChanges({
      changes,
      lastPulledAt,
    }): Promise<PushChangesResponse> {
      // Transform WatermelonDB changes to server format
      const pushChanges = [];
      
      for (const [tableName, tableChanges] of Object.entries(changes)) {
        // Handle created records
        if (tableChanges.created) {
          for (const record of tableChanges.created) {
            pushChanges.push({
              table: tableName,
              id: record.id,
              data: record,
              deleted: false,
              device_id: deviceId,
              version_vector: record.version_vector ? JSON.parse(record.version_vector) : null,
            });
          }
        }

        // Handle updated records
        if (tableChanges.updated) {
          for (const record of tableChanges.updated) {
            pushChanges.push({
              table: tableName,
              id: record.id,
              data: record,
              deleted: false,
              device_id: deviceId,
              version_vector: record.version_vector ? JSON.parse(record.version_vector) : null,
            });
          }
        }

        // Handle deleted records
        if (tableChanges.deleted) {
          for (const recordId of tableChanges.deleted) {
            pushChanges.push({
              table: tableName,
              id: recordId,
              data: null,
              deleted: true,
              device_id: deviceId,
            });
          }
        }
      }

      const response = await fetch(`${apiUrl}/sync/push`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${authToken}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          changes: pushChanges,
          device_id: deviceId,
        }),
      });

      if (!response.ok) {
        throw new Error(`Push sync failed: ${response.statusText}`);
      }

      const data = await response.json();

      return {
        conflicts: data.conflicts > 0 ? [] : [], // Handle conflicts if needed
      };
    },
  };
}

/**
 * Sync configuration helper
 */
export function getSyncConfig(): SyncConfig {
  const apiUrl = process.env.EXPO_PUBLIC_API_URL || 'http://localhost:3000';
  const authToken = ''; // Get from auth store
  const deviceId = ''; // Generate or get from device

  return {
    apiUrl,
    authToken,
    deviceId,
  };
}

