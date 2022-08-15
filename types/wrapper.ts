import {definitions} from "./supabase";

export const enum LocationName {
    Gara = "gara",
    Boromir = "boromir"
}

export type Location = definitions['locations']

export type GameTable = definitions['mese']

export type Profile = definitions['profiles']

export type Reservation = definitions['rezervari']

export function getStartTime(reservation: Reservation): Date {
    return new Date(reservation.start_date);
}

export function getEndDateDuration(date: Date, hours: number): Date {
    return new Date(date.getTime() + hours * 1000 * 60 * 60);
}

export function getEndTime(reservation: Reservation): Date {
    return getEndDateDuration(new Date(reservation.start_date), reservation.duration);
}

export const enum ReservationStatus {
    Approved = "approved",
    Cancelled = "cancelled",
}
