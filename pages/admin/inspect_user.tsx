import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import {useRouter} from "next/router";
import {useProfile} from "../../components/ProfileProvider";
import React, {useEffect, useMemo, useState} from "react";
import {MemberTypes, Profile, Reservation, ReservationStatus} from "../../types/wrapper";
import {Card, Center, Indicator, Loader, Select, SimpleGrid, Space, Stack, Text} from "@mantine/core";
import {RangeCalendar} from "@mantine/dates";
import 'dayjs/locale/ro';
import {dateToISOString} from "../../utils/date";

export default function InspectUser() {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()
    const profileData = useProfile()

    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [reservations, setReservations] = useState<Reservation[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
    const [dateRange, setDateRange] = useState<[Date | null, Date | null]>([null, null]);
    const [startRange, endRange] = dateRange;

    useEffect(() => {
        if ((!profileData.isLoading && profileData.profile == null) || profileData.profile?.role !== MemberTypes.Fondator)
            router.back()
    }, [profileData, router])

    useEffect(() => {
        supabase.from('profiles').select('*')
            .order('name', {ascending: true})
            .then(value => {
                if (value.data != null) {
                    setAllProfiles(value.data)
                    setIsLoading(false)
                }
            })

        supabase.from('rezervari').select('*')
            .eq('status', ReservationStatus.Approved)
            .order('start_date', {ascending: false})
            .order('start_hour', {ascending: true})
            .then(value => {
                if (value.data != null) {
                    setReservations(value.data)
                }
            })
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    const selectedProfile = useMemo(
        () => allProfiles.find(it => it.id === selectedProfileId) || null,
        [allProfiles, selectedProfileId]
    )
    const filteredReservations = useMemo(
        () => {
            if (startRange == null || endRange == null) {
                return []
            }
            return reservations.filter(it => it.user_id === selectedProfileId && new Date(it.start_date) >= startRange && new Date(it.start_date) <= endRange)
        },
        [reservations, selectedProfileId, startRange, endRange]
    )

    if (profileData.isLoading || isLoading)
        return <Center> <Loader/> </Center>;

    if (profileData.profile == null)
        return (<></>)

    return <>
        <SimpleGrid
            cols={1}
            breakpoints={[
                {minWidth: 1120, cols: 2},
            ]}>

            <Stack p={'md'}>
                <Select
                    label="Alege un utilizator"
                    placeholder="Utilizator"
                    searchable
                    transition="pop-top-left"
                    transitionDuration={80}
                    transitionTimingFunction="ease"
                    data={allProfiles.map(profile => ({value: profile.id, label: profile.name}))}
                    value={selectedProfileId}
                    onChange={setSelectedProfileId}
                />

                <RangeCalendar
                    value={dateRange}
                    onChange={setDateRange}
                    size={"lg"}
                    locale="ro"
                    renderDay={(date) => {
                        return (
                            <Indicator size={8} color="green" offset={8}
                                       disabled={!filteredReservations.some(it => it.start_date === dateToISOString(date))}>
                                <div>{date.getDate()}</div>
                            </Indicator>
                        );
                    }}
                />
            </Stack>

            <Stack p={'md'}>
                {selectedProfile &&
                    SelectedUserReservations(selectedProfile, filteredReservations)
                }
            </Stack>
        </SimpleGrid>

        <Space h="xl"/>
    </>
}

function SelectedUserReservations(profile: Profile, reservations: Reservation[]) {
    console.log(reservations)
    return <>
        <Text size={'xl'}>Total: {reservations.length}</Text>

        {
            reservations.map(reservation => {
                return <Card key={reservation.id}>
                    <Stack>
                        <Text weight={900}>{(new Date(reservation.start_date)).toLocaleDateString('ro-RO')}</Text>

                        <Text>De
                            la <b>{reservation.start_hour}:{'00'}</b> la <b>{reservation.start_hour + reservation.duration}:{'00'}</b></Text>
                    </Stack>
                </Card>
            })
        }
    </>
}
