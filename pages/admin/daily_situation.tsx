import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import React, {useEffect, useMemo, useState} from "react";
import {Profile, Reservation} from "../../types/wrapper";
import {Button, Divider, Grid, Group, Space, Stack, Text} from "@mantine/core";
import {DatePicker} from "@mantine/dates";
import 'dayjs/locale/ro';
import {dateToISOString} from "../../utils/date";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import useProfilesQuery from "../../hooks/useProfilesQuery";
import AdminScaffold from "../../components/AdminInput/AdminScaffold";
import useGuestsQuery from "../../hooks/useGuestsQuery";

const groupBy = <T, K extends keyof any>(arr: T[], key: (i: T) => K) =>
    arr.reduce((groups, item) => {
        (groups[key(item)] ||= []).push(item);
        return groups;
    }, {} as Record<K, T[]>);

interface ReservationAndGuest {
    id: string,
    name: string,
    start_hour: number,
    is_guest: boolean,
}

export default function DailySituationPage() {
    useExitIfNotFounder();

    const supabase = useSupabaseClient<Database>()

    const {data: allProfiles} = useProfilesQuery()
    const [date, setDate] = useState<Date | undefined>(undefined);
    const [reservations, setReservations] = useState<Reservation[]>([])
    const {data: guests} = useGuestsQuery(date || null)

    useEffect(() => {
        if (date) {
            supabase.from('rezervari').select('*')
                .eq('cancelled', false)
                .eq('start_date', dateToISOString(date))
                .then(value => {
                    if (value.data != null) {
                        setReservations(value.data)
                    }
                })
        }
    }, [date, supabase])

    const groupedReservations = useMemo(() => {
        const mapped_reservations: ReservationAndGuest[] = reservations.map(res => {
            return {
                id: res.id,
                name: allProfiles?.find(profile => profile.id == res.user_id)?.name || "Necunoscut",
                start_hour: res.start_hour,
                is_guest: false
            }
        })
        const mapped_guests: ReservationAndGuest[] = guests?.map(guest => {
            return {id: guest.created_at, name: guest.guest_name, start_hour: guest.start_hour, is_guest: true}
        }) || []
        return groupBy(mapped_reservations.concat(mapped_guests), reservation => reservation.start_hour)
    }, [reservations, guests, allProfiles])

    return <AdminScaffold>
        <Grid
            grow={true}
            columns={4}
        >

            <Grid.Col span={"auto"}>
                <DatePicker
                    value={date}
                    onChange={(newDate) => setDate(newDate || undefined)}
                    size={"lg"}
                    locale="ro"
                />
            </Grid.Col>

            <Grid.Col span={2}>
                <Stack p={'md'}>
                    {date ?
                        SelectedDateReservations(groupedReservations)
                        :
                        <Text size={'xl'}>Selectează o dată pentru a vedea rezervările</Text>
                    }
                </Stack>
            </Grid.Col>
        </Grid>
    </AdminScaffold>
}

function SelectedDateReservations(reservations: Record<number, ReservationAndGuest[]>) {
    return <>
        {Object.entries(reservations).map(([key, reservation]) => {
            return <React.Fragment key={key}>
                <Group>
                    <Text><b>Ora {key}:</b></Text>

                    {
                        reservation.map((user, index) => {
                            return <Button key={user.id} radius={'xl'} color={user.is_guest ? 'cyan' : 'gray'}
                                           size={'xs'}>{index + 1}. {user.name}</Button>
                        })
                    }
                </Group>

                <Divider mt={'md'} mb={'md'}/>
            </React.Fragment>
        })
        }
    </>
}
