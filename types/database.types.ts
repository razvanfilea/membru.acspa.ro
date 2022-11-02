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
          user_id: string
          guest_name: string
          date: string
          start_hour: number
        }
        Insert: {
          created_at?: string
          user_id?: string
          guest_name: string
          date: string
          start_hour: number
        }
        Update: {
          created_at?: string
          user_id?: string
          guest_name?: string
          date?: string
          start_hour?: number
        }
      }
      locations: {
        Row: {
          name: string
          reservation_duration: number
          max_reservations: number
          start_hour: number
          end_hour: number
          weekend_reservation_duration: number
          weekend_start_hour: number
          weekend_end_hour: number
        }
        Insert: {
          name: string
          reservation_duration: number
          max_reservations: number
          start_hour: number
          end_hour: number
          weekend_reservation_duration?: number
          weekend_start_hour?: number
          weekend_end_hour?: number
        }
        Update: {
          name?: string
          reservation_duration?: number
          max_reservations?: number
          start_hour?: number
          end_hour?: number
          weekend_reservation_duration?: number
          weekend_start_hour?: number
          weekend_end_hour?: number
        }
      }
      member_types: {
        Row: {
          type: string
        }
        Insert: {
          type: string
        }
        Update: {
          type?: string
        }
      }
      mese: {
        Row: {
          id: string
          name: string
          location: string
          type: string
          color: string
          has_robot: boolean
        }
        Insert: {
          id: string
          name: string
          location: string
          type: string
          color: string
          has_robot?: boolean
        }
        Update: {
          id?: string
          name?: string
          location?: string
          type?: string
          color?: string
          has_robot?: boolean
        }
      }
      profiles: {
        Row: {
          id: string
          name: string
          member_type: string
          has_key: boolean
        }
        Insert: {
          id?: string
          name: string
          member_type?: string
          has_key?: boolean
        }
        Update: {
          id?: string
          name?: string
          member_type?: string
          has_key?: boolean
        }
      }
      reservations_restrictions: {
        Row: {
          date: string
          start_hour: number
          message: string
          user_id: string
        }
        Insert: {
          date: string
          start_hour: number
          message: string
          user_id?: string
        }
        Update: {
          date?: string
          start_hour?: number
          message?: string
          user_id?: string
        }
      }
      rezervari: {
        Row: {
          id: string
          created_at: string
          user_id: string
          table_id: string
          start_date: string
          status: string
          start_hour: number
          duration: number
        }
        Insert: {
          id?: string
          created_at?: string
          user_id: string
          table_id: string
          start_date: string
          status: string
          start_hour: number
          duration: number
        }
        Update: {
          id?: string
          created_at?: string
          user_id?: string
          table_id?: string
          start_date?: string
          status?: string
          start_hour?: number
          duration?: number
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
