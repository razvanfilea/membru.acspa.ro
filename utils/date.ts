import {Reservation} from "../types/wrapper";

export function addDaysToDate(date: Date, days: number): Date {
    const result = new Date(date);
    result.setDate(result.getDate() + days);
    return result;
}

export function dateToISOString(date: Date): string {
    const month = ("0" + (date.getMonth() + 1)).slice(-2)
    const day = ("0" + date.getDate()).slice(-2)
    const year = date.getFullYear()

    return year + "-" + month + "-" + day;
}

export function isWeekend(date: Date): boolean {
    return date.getDay() === 6 || date.getDay() === 0
}

export function isReservationCancelable(reservation: Reservation): boolean {
    const reservationDate = new Date(reservation.start_date)
    const now = new Date();
    return reservationDate.getTime() > now.getTime() ||
        (reservation.start_date == dateToISOString(now) && reservation.start_hour > now.getHours())
}
