export type NotificationChannel = 'email' | 'telegram' | 'discord' | 'push';
export type AlertType = 'health_factor_low' | 'approaching_liquidation' | 'position_liquidated' | 'liquidatable_position' | 'price_alert';
export type DeliveryStatus = 'pending' | 'sent' | 'delivered' | 'read' | 'failed';

export interface NotificationPreference {
  userId: string;
  channel: NotificationChannel;
  alertType: AlertType;
  enabled: boolean;
  threshold?: number;
}

export interface NotificationMessage {
  id: string;
  userId: string;
  channel: NotificationChannel;
  alertType: AlertType;
  title: string;
  body: string;
  data?: Record<string, unknown>;
  status: DeliveryStatus;
  createdAt: string;
  sentAt?: string;
  deliveredAt?: string;
  readAt?: string;
}

export interface SubscribeRequest {
  channel: NotificationChannel;
  recipient: string;
  alertTypes: AlertType[];
}

export interface PreferenceUpdate {
  channel: NotificationChannel;
  alertType: AlertType;
  enabled: boolean;
  threshold?: number;
}

export interface NotificationHistoryQuery {
  alertType?: AlertType;
  channel?: NotificationChannel;
  limit?: number;
  cursor?: string;
}

export interface NotificationTemplate {
  id: string;
  alertType: AlertType;
  titleTemplate: string;
  bodyTemplate: string;
  variables: string[];
}
