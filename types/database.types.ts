export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json }
  | Json[]

export interface Database {
  public: {
    Tables: {
      guest_invites: {
        Row: {
          created_at: string
          guest_name: string
          special: boolean
          start_date: string
          start_hour: number
          user_id: string
        }
        Insert: {
          created_at?: string
          guest_name: string
          special: boolean
          start_date: string
          start_hour: number
          user_id?: string
        }
        Update: {
          created_at?: string
          guest_name?: string
          special?: boolean
          start_date?: string
          start_hour?: number
          user_id?: string
        }
      }
      locations: {
        Row: {
          end_hour: number
          max_reservations: number
          name: string
          reservation_duration: number
          start_hour: number
          weekend_end_hour: number
          weekend_reservation_duration: number
          weekend_start_hour: number
        }
        Insert: {
          end_hour: number
          max_reservations: number
          name: string
          reservation_duration: number
          start_hour: number
          weekend_end_hour?: number
          weekend_reservation_duration?: number
          weekend_start_hour?: number
        }
        Update: {
          end_hour?: number
          max_reservations?: number
          name?: string
          reservation_duration?: number
          start_hour?: number
          weekend_end_hour?: number
          weekend_reservation_duration?: number
          weekend_start_hour?: number
        }
      }
      member_roles: {
        Row: {
          role: string
        }
        Insert: {
          role: string
        }
        Update: {
          role?: string
        }
      }
      mese: {
        Row: {
          color: string
          has_robot: boolean
          id: string
          location: string
          name: string
          type: string
        }
        Insert: {
          color: string
          has_robot?: boolean
          id: string
          location: string
          name: string
          type: string
        }
        Update: {
          color?: string
          has_robot?: boolean
          id?: string
          location?: string
          name?: string
          type?: string
        }
      }
      profiles: {
        Row: {
          has_key: boolean
          id: string
          name: string
          role: string
        }
        Insert: {
          has_key?: boolean
          id?: string
          name: string
          role?: string
        }
        Update: {
          has_key?: boolean
          id?: string
          name?: string
          role?: string
        }
      }
      reservations_restrictions: {
        Row: {
          date: string
          message: string
          start_hour: number
          user_id: string
        }
        Insert: {
          date: string
          message: string
          start_hour: number
          user_id?: string
        }
        Update: {
          date?: string
          message?: string
          start_hour?: number
          user_id?: string
        }
      }
      rezervari: {
        Row: {
          created_at: string
          duration: number
          id: string
          start_date: string
          start_hour: number
          status: string
          table_id: string
          user_id: string
        }
        Insert: {
          created_at?: string
          duration: number
          id?: string
          start_date: string
          start_hour: number
          status: string
          table_id: string
          user_id: string
        }
        Update: {
          created_at?: string
          duration?: number
          id?: string
          start_date?: string
          start_hour?: number
          status?: string
          table_id?: string
          user_id?: string
        }
      }
      rezervari_status: {
        Row: {
          status: string
        }
        Insert: {
          status: string
        }
        Update: {
          status?: string
        }
      }
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      create_reservation: {
        Args: {
          table_id_input: string
          start_date_input: string
          start_hour_input: number
        }
        Returns: string
      }
    }
    Enums: {
      [_ in never]: never
    }
  }
}
