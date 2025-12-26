import { SyncPullArgs, SyncPushArgs, SyncPullResult, SyncPushResult } from '@nozbe/watermelondb/sync';
import { api } from '../api/client';

/**
 * WatermelonDB Sync Adapter for GigPilot
 * 
 * This adapter connects WatermelonDB to the Rust backend's sync endpoints.
 * It handles pull/push synchronization compatible with the custom sync protocol.
 */

/**
 * Pull changes from the server.
 * 
 * @param args - Sync pull arguments from WatermelonDB
 * @returns Changes grouped by table
 */
export async function pullChanges(args: SyncPullArgs): Promise<SyncPullResult> {
  const { lastPulledAt } = args;
  
  try {
    const response = await api.get('/sync/pull', {
      params: {
        last_pulled_at: lastPulledAt ? new Date(lastPulledAt).toISOString() : null,
        device_id: getDeviceId(),
      },
    });
    
    const { changes, timestamp } = response.data;
    
    // Convert WatermelonDB format to our format
    // The backend returns: { "invoices": { "created": [...], "updated": [...], "deleted": [...] } }
    return {
      changes,
      timestamp: new Date(timestamp).getTime(),
    };
  } catch (error) {
    console.error('Pull sync failed:', error);
    throw error;
  }
}

/**
 * Push changes to the server.
 * 
 * @param args - Sync push arguments from WatermelonDB
 * @returns Push result with applied/conflicted counts
 */
export async function pushChanges(args: SyncPushArgs): Promise<SyncPushResult> {
  const { changes, lastPulledAt } = args;
  
  try {
    // Convert WatermelonDB changes format to our push format
    const pushChanges = [];
    
    for (const [table, tableChanges] of Object.entries(changes)) {
      // Handle created records
      if (tableChanges.created) {
        for (const record of tableChanges.created) {
          pushChanges.push({
            table,
            id: record.id,
            data: record,
            deleted: false,
            device_id: getDeviceId(),
            version_vector: record.version_vector ? JSON.parse(record.version_vector) : null,
          });
        }
      }
      
      // Handle updated records
      if (tableChanges.updated) {
        for (const record of tableChanges.updated) {
          pushChanges.push({
            table,
            id: record.id,
            data: record,
            deleted: false,
            device_id: getDeviceId(),
            version_vector: record.version_vector ? JSON.parse(record.version_vector) : null,
          });
        }
      }
      
      // Handle deleted records
      if (tableChanges.deleted) {
        for (const recordId of tableChanges.deleted) {
          pushChanges.push({
            table,
            id: recordId,
            data: null,
            deleted: true,
            device_id: getDeviceId(),
            version_vector: null,
          });
        }
      }
    }
    
    const response = await api.post('/sync/push', {
      changes: pushChanges,
      device_id: getDeviceId(),
    });
    
    const { applied, conflicts, conflicted_ids, timestamp } = response.data;
    
    return {
      applied,
      conflicts,
      conflicted_ids,
      timestamp: new Date(timestamp).getTime(),
    };
  } catch (error) {
    console.error('Push sync failed:', error);
    throw error;
  }
}

/**
 * Gets or creates a unique device ID for this client.
 * 
 * @returns Device ID string
 */
function getDeviceId(): string {
  // In a real implementation, this would use AsyncStorage or similar
  // to persist the device ID across app restarts
  if (typeof window !== 'undefined') {
    let deviceId = localStorage.getItem('gigpilot_device_id');
    if (!deviceId) {
      deviceId = `device_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
      localStorage.setItem('gigpilot_device_id', deviceId);
    }
    return deviceId;
  }
  
  // Fallback for React Native
  return `device_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

